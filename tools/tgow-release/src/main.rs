#![forbid(unsafe_code)]

mod build;
mod options;

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ed25519_compact::{KeyPair, Seed};
use tgow_core::catalog::SIGNATURE_SUFFIX;
use tgow_core::checksum::Digest;
use tgow_core::manifest::{Identity, Manifest};
use tgow_core::trust::TrustedKeys;

use crate::options::Options;

const USAGE: &str = "Usage: tgow-release <command> [options]

Commands:
  keygen --out <file>
      Create an Ed25519 key pair. The secret seed goes to <file>, the public key is printed.
  manifest --app <id> --name <name> --version <x.y.z> [--channel stable|beta]
           [--tagline-de <text>] [--tagline-en <text>] [--notes-de <file>] [--notes-en <file>]
           [--icon <file>] [--hero <file>] [--accent #rrggbb] [--launch <path>]
           [--installer inno|nsis] [--out <dir>] <asset>...
      Write tgow-release.json and SHA256SUMS.txt for the given release assets.
  sign --key <file> <manifest>
      Sign the manifest with the seed from <file> or from TGOW_SIGNING_KEY.
  verify --public <hex> <manifest>
      Check the signature, the manifest fields and the hashes of all assets next to it.
";

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    match run(&arguments) {
        Ok(output) => {
            let _ = io::stdout().lock().write_all(output.as_bytes());
            ExitCode::SUCCESS
        }
        Err(message) => {
            let _ = writeln!(io::stderr().lock(), "tgow-release: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[String]) -> Result<String, String> {
    let (command, rest) = arguments.split_first().ok_or_else(|| USAGE.to_owned())?;
    let options = Options::parse(rest)?;
    match command.as_str() {
        "keygen" => keygen(&options),
        "manifest" => build::manifest(&options),
        "sign" => sign(&options),
        "verify" => verify(&options),
        _ => Err(USAGE.to_owned()),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
}

fn decode(text: &str) -> Option<Vec<u8>> {
    let text = text.trim();
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).ok())
        .collect()
}

fn keygen(options: &Options) -> Result<String, String> {
    let out = options.path("out")?;
    if out.exists() {
        return Err(format!("{} already exists", out.display()));
    }
    let pair = KeyPair::from_seed(Seed::generate());
    fs::write(&out, hex(pair.sk.seed().as_ref())).map_err(|error| error.to_string())?;
    Ok(format!("{}\n", hex(pair.pk.as_ref())))
}

fn key_pair(options: &Options) -> Result<KeyPair, String> {
    let text = match options.value("key") {
        Some(file) => fs::read_to_string(file).map_err(|error| error.to_string())?,
        None => env::var("TGOW_SIGNING_KEY")
            .map_err(|error| format!("no key given, use --key or TGOW_SIGNING_KEY ({error})"))?,
    };
    let seed = decode(&text)
        .and_then(|bytes| Seed::from_slice(&bytes).ok())
        .ok_or("the key is not a 32 byte hex seed")?;
    Ok(KeyPair::from_seed(seed))
}

fn sign(options: &Options) -> Result<String, String> {
    let manifest = options.single_file()?;
    let pair = key_pair(options)?;
    let bytes = fs::read(&manifest).map_err(|error| error.to_string())?;
    let signature = hex(pair.sk.sign(&bytes, None).as_ref());
    let target = signature_path(&manifest);
    fs::write(&target, &signature).map_err(|error| error.to_string())?;
    Ok(format!("{}\n", target.display()))
}

fn signature_path(manifest: &Path) -> PathBuf {
    let mut name = manifest.as_os_str().to_owned();
    name.push(SIGNATURE_SUFFIX);
    PathBuf::from(name)
}

fn verify(options: &Options) -> Result<String, String> {
    let manifest_file = options.single_file()?;
    let public = options.value("public").ok_or("--public is required")?;
    let keys = TrustedKeys::from_hex(&[public]).map_err(|error| error.to_string())?;
    let bytes = fs::read(&manifest_file).map_err(|error| error.to_string())?;
    let signature =
        fs::read_to_string(signature_path(&manifest_file)).map_err(|error| error.to_string())?;
    keys.verify(&bytes, &signature)
        .map_err(|error| error.to_string())?;
    let app = serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|value| {
            value
                .get("app")
                .and_then(|app| app.as_str())
                .map(str::to_owned)
        })
        .ok_or("manifest has no app field")?;
    let manifest = Manifest::parse(
        &bytes,
        Identity {
            app: &app,
            name: &app,
        },
    )
    .map_err(|error| error.to_string())?;
    let folder = manifest_file.parent().unwrap_or_else(|| Path::new("."));
    let mut report = String::new();
    for asset in &manifest.assets {
        let file = folder.join(&asset.file);
        let digest = fs::File::open(&file)
            .and_then(Digest::of_reader)
            .map_err(|error| format!("{}: {error}", asset.file))?;
        if digest != asset.sha256 {
            return Err(format!("{} does not match its checksum", asset.file));
        }
        let _ = writeln!(report, "ok  {}", asset.file);
    }
    let _ = writeln!(
        report,
        "signature valid, {} {} verified",
        manifest.app, manifest.version
    );
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::{decode, hex, run};

    #[test]
    fn hex_round_trips() {
        let bytes = [0_u8, 15, 16, 255];
        assert_eq!(decode(&hex(&bytes)), Some(bytes.to_vec()));
        assert_eq!(decode("abc"), None);
        assert_eq!(decode("zz"), None);
    }

    #[test]
    fn creates_signs_and_verifies_a_release() {
        let folder = std::env::temp_dir().join(format!("tgow-release-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let asset = folder.join("demo-1.2.0-windows-x64-portable.exe");
        std::fs::write(&asset, b"MZ demo").unwrap();
        let key = folder.join("release.key");
        let arg = |text: &str| text.to_owned();
        let public = run(&[arg("keygen"), arg("--out"), key.display().to_string()]).unwrap();
        run(&[
            arg("manifest"),
            arg("--app"),
            arg("demo"),
            arg("--name"),
            arg("Demo"),
            arg("--version"),
            arg("1.2.0"),
            arg("--out"),
            folder.display().to_string(),
            asset.display().to_string(),
        ])
        .unwrap();
        let manifest = folder.join("tgow-release.json");
        run(&[
            arg("sign"),
            arg("--key"),
            key.display().to_string(),
            manifest.display().to_string(),
        ])
        .unwrap();
        let report = run(&[
            arg("verify"),
            arg("--public"),
            public.trim().to_owned(),
            manifest.display().to_string(),
        ])
        .unwrap();
        assert!(report.contains("signature valid"), "{report}");
        std::fs::write(&asset, b"MZ changed").unwrap();
        assert!(
            run(&[
                arg("verify"),
                arg("--public"),
                public.trim().to_owned(),
                manifest.display().to_string()
            ])
            .is_err()
        );
        std::fs::remove_dir_all(folder).unwrap();
    }
}

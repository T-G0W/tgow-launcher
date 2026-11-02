use std::collections::BTreeMap;
use std::path::PathBuf;

pub(crate) struct Options {
    values: BTreeMap<String, String>,
    pub(crate) files: Vec<PathBuf>,
}

impl Options {
    pub(crate) fn parse(arguments: &[String]) -> Result<Self, String> {
        let mut values = BTreeMap::new();
        let mut files = Vec::new();
        let mut iter = arguments.iter();
        while let Some(argument) = iter.next() {
            if let Some(name) = argument.strip_prefix("--") {
                let value = iter
                    .next()
                    .ok_or_else(|| format!("--{name} needs a value"))?;
                values.insert(name.to_owned(), value.clone());
            } else {
                files.push(PathBuf::from(argument));
            }
        }
        Ok(Self { values, files })
    }

    pub(crate) fn value(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }

    pub(crate) fn required(&self, name: &str) -> Result<&str, String> {
        self.value(name)
            .ok_or_else(|| format!("--{name} is required"))
    }

    pub(crate) fn path(&self, name: &str) -> Result<PathBuf, String> {
        self.required(name).map(PathBuf::from)
    }

    pub(crate) fn single_file(&self) -> Result<PathBuf, String> {
        match self.files.as_slice() {
            [file] => Ok(file.clone()),
            _ => Err("expected exactly one file".to_owned()),
        }
    }
}

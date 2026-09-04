use std::ops::Deref;

use serde::Deserialize;

use crate::error::ApplicationError;

pub struct ValidCatedoryPath(Vec<String>);

impl ValidCatedoryPath {
    pub fn into_inner(self) -> Vec<String> {
        self.0
    }
}

impl Deref for ValidCatedoryPath {
    type Target = [String];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Vec<String>> for ValidCatedoryPath {
    type Error = ApplicationError;

    fn try_from(category_path: Vec<String>) -> Result<Self, Self::Error> {
        if category_path.is_empty() || category_path.iter().any(|name| name.trim().is_empty()) {
            return Err(ApplicationError::ValidationFailed {
                message: "The category path cannot be empty".to_owned(),
            });
        }

        Ok(Self(category_path))
    }
}

impl<'de> Deserialize<'de> for ValidCatedoryPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let category_path = <Vec<String>>::deserialize(deserializer)?;

        ValidCatedoryPath::try_from(category_path).map_err(serde::de::Error::custom)
    }
}

use serde::Deserialize;

use crate::error::ApplicationError;

pub fn valid_timestamp_start() -> i64 {
    // TODO: it can be controlled by configuration value.
    0
}

#[derive(Clone)]
pub struct ValidTimestamp(chrono::DateTime<chrono::Utc>);

impl ValidTimestamp {
    pub fn into_inner(self) -> chrono::DateTime<chrono::Utc> {
        self.0
    }
}

impl TryFrom<i64> for ValidTimestamp {
    type Error = ApplicationError;

    fn try_from(timestamp_ms: i64) -> Result<Self, Self::Error> {
        /// 5 min.
        ///
        /// It was made for avoiding a possible time difference between client and server sides.
        const MAX_FUTURE_SKEW: i64 = 5 * 60 * 1000;

        if timestamp_ms < valid_timestamp_start() {
            return Err(ApplicationError::ValidationFailed {
                message: "The \"created_at_ms\" field cannot be below 0.".to_owned(),
            });
        }

        if timestamp_ms > chrono::Utc::now().timestamp_millis() + MAX_FUTURE_SKEW {
            return Err(ApplicationError::ValidationFailed {
                message: "The \"created_at_ms\" field cannot be above than current time."
                    .to_owned(),
            });
        }

        Ok(Self(
            chrono::DateTime::from_timestamp_millis(timestamp_ms)
                .expect("The \"created_at_ms\" timestamp in must be valid"),
        ))
    }
}

impl<'de> Deserialize<'de> for ValidTimestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let timestamp = i64::deserialize(deserializer)?;

        ValidTimestamp::try_from(timestamp).map_err(serde::de::Error::custom)
    }
}

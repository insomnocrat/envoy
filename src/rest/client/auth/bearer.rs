use chrono::{self, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Access {
    pub(crate) token: String,
    pub(crate) expiration: chrono::DateTime<Utc>,
}

impl From<AccessTokenResponse> for Access {
    fn from(atr: AccessTokenResponse) -> Self {
        let duration = Duration::seconds(atr.expires_in as i64);
        let expiration = Utc::now() + duration;

        Self {
            token: atr.access_token,
            expiration,
        }
    }
}

impl Access {
    pub fn is_expired(&self) -> bool {
        let current_time = Utc::now().naive_utc();
        let time_elapsed = current_time.signed_duration_since(self.expiration.naive_utc());

        time_elapsed.num_minutes() >= 3
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AccessTokenResponse {
    #[serde(alias = "accessToken")]
    pub access_token: String,
    #[serde(default)]
    pub token_type: String,
    #[serde(default = "default_expiry")]
    pub expires_in: u64,
}

fn default_expiry() -> u64 {
    (Utc::now().naive_utc() + Duration::minutes(30)).second() as u64
}

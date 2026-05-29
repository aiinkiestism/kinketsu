use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input DTO for creating a category. The server fills in `id` and timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCategory {
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
}

impl NewCategory {
    #[must_use]
    pub fn into_category(self) -> Category {
        let now = Utc::now();
        Category {
            id: Uuid::now_v7(),
            name: self.name,
            icon: self.icon,
            color: self.color,
            created_at: now,
            updated_at: now,
        }
    }
}

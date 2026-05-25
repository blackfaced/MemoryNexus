//! Cognitive Space database operations

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CognitiveSpaceDb {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_user_id: Uuid,
    pub default_lens_id: Option<Uuid>,
    #[serde(default = "default_space_type")]
    pub space_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_space_type() -> String {
    "personal".to_string()
}

#[derive(Debug, Clone)]
pub struct CreateCognitiveSpace {
    pub name: String,
    pub description: Option<String>,
    pub owner_user_id: Uuid,
    pub space_type: CognitiveSpaceType,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CognitiveSpaceType {
    #[default]
    Personal,
    Family,
    Project,
    Organization,
}

impl CognitiveSpaceType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Family => "family",
            Self::Project => "project",
            Self::Organization => "organization",
        }
    }
}

impl std::fmt::Display for CognitiveSpaceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpaceMemberRole {
    Owner,
    Editor,
    Viewer,
}

impl SpaceMemberRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Editor => "editor",
            Self::Viewer => "viewer",
        }
    }

    pub fn can_write(self) -> bool {
        matches!(self, Self::Owner | Self::Editor)
    }

    pub fn can_manage_members(self) -> bool {
        matches!(self, Self::Owner)
    }

    pub fn is_invitable(self) -> bool {
        matches!(self, Self::Editor | Self::Viewer)
    }
}

impl std::fmt::Display for SpaceMemberRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SpaceMemberRole {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "owner" => Ok(Self::Owner),
            "editor" => Ok(Self::Editor),
            "viewer" => Ok(Self::Viewer),
            other => Err(format!("unknown space member role: {other}")),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CognitiveSpaceMemberDb {
    pub space_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

impl CognitiveSpaceMemberDb {
    pub fn parsed_role(&self) -> Option<SpaceMemberRole> {
        self.role.parse().ok()
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CognitiveSpaceInviteDb {
    pub id: Uuid,
    pub space_id: Uuid,
    pub code: String,
    pub role: String,
    pub created_by: Uuid,
    pub accepted_by: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateSpaceInvite {
    pub space_id: Uuid,
    pub code: String,
    pub role: SpaceMemberRole,
    pub created_by: Uuid,
    pub expires_in_days: Option<i64>,
}

#[async_trait::async_trait]
pub trait CognitiveSpaceRepository: Send + Sync {
    async fn create(&self, space: CreateCognitiveSpace) -> Result<CognitiveSpaceDb, Error>;
    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<CognitiveSpaceDb>, Error>;
    async fn find_for_user(
        &self,
        space_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CognitiveSpaceDb>, Error>;
    async fn default_for_user(&self, user_id: Uuid) -> Result<Option<CognitiveSpaceDb>, Error>;
    async fn ensure_default_for_user(
        &self,
        user_id: Uuid,
        username: &str,
    ) -> Result<CognitiveSpaceDb, Error>;
    async fn find_member(
        &self,
        space_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CognitiveSpaceMemberDb>, Error>;
    async fn list_members_for_user(
        &self,
        space_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<CognitiveSpaceMemberDb>, Error>;
    async fn update_member_role(
        &self,
        space_id: Uuid,
        user_id: Uuid,
        role: SpaceMemberRole,
    ) -> Result<Option<CognitiveSpaceMemberDb>, Error>;
    async fn create_invite(
        &self,
        invite: CreateSpaceInvite,
    ) -> Result<CognitiveSpaceInviteDb, Error>;
    async fn accept_invite(
        &self,
        code: &str,
        user_id: Uuid,
    ) -> Result<Option<CognitiveSpaceInviteDb>, Error>;
}

pub struct PostgresCognitiveSpaceRepository {
    pool: PgPool,
}

impl PostgresCognitiveSpaceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl CognitiveSpaceRepository for PostgresCognitiveSpaceRepository {
    async fn create(&self, space: CreateCognitiveSpace) -> Result<CognitiveSpaceDb, Error> {
        let mut tx = self.pool.begin().await?;

        let created = sqlx::query_as::<_, CognitiveSpaceDb>(
            r#"
            INSERT INTO cognitive_spaces (name, description, owner_user_id, space_type)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(&space.name)
        .bind(&space.description)
        .bind(space.owner_user_id)
        .bind(space.space_type.to_string())
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO cognitive_space_members (space_id, user_id, role)
            VALUES ($1, $2, 'owner')
            ON CONFLICT (space_id, user_id) DO NOTHING
            "#,
        )
        .bind(created.id)
        .bind(space.owner_user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(created)
    }

    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<CognitiveSpaceDb>, Error> {
        sqlx::query_as::<_, CognitiveSpaceDb>(
            r#"
            SELECT s.*
            FROM cognitive_spaces s
            INNER JOIN cognitive_space_members m ON m.space_id = s.id
            WHERE m.user_id = $1
            ORDER BY s.created_at ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn find_for_user(
        &self,
        space_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CognitiveSpaceDb>, Error> {
        sqlx::query_as::<_, CognitiveSpaceDb>(
            r#"
            SELECT s.*
            FROM cognitive_spaces s
            INNER JOIN cognitive_space_members m ON m.space_id = s.id
            WHERE s.id = $1 AND m.user_id = $2
            "#,
        )
        .bind(space_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn default_for_user(&self, user_id: Uuid) -> Result<Option<CognitiveSpaceDb>, Error> {
        sqlx::query_as::<_, CognitiveSpaceDb>(
            r#"
            SELECT s.*
            FROM cognitive_spaces s
            INNER JOIN cognitive_space_members m ON m.space_id = s.id
            WHERE m.user_id = $1
            ORDER BY
                CASE WHEN s.owner_user_id = $1 THEN 0 ELSE 1 END,
                s.created_at ASC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn ensure_default_for_user(
        &self,
        user_id: Uuid,
        username: &str,
    ) -> Result<CognitiveSpaceDb, Error> {
        if let Some(space) = self.default_for_user(user_id).await? {
            return Ok(space);
        }

        self.create(CreateCognitiveSpace {
            name: format!("{username} Personal Space"),
            description: Some("Default personal cognitive space".to_string()),
            owner_user_id: user_id,
            space_type: CognitiveSpaceType::Personal,
        })
        .await
    }

    async fn find_member(
        &self,
        space_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CognitiveSpaceMemberDb>, Error> {
        sqlx::query_as::<_, CognitiveSpaceMemberDb>(
            r#"
            SELECT space_id, user_id, role, created_at
            FROM cognitive_space_members
            WHERE space_id = $1 AND user_id = $2
            "#,
        )
        .bind(space_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn list_members_for_user(
        &self,
        space_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<CognitiveSpaceMemberDb>, Error> {
        sqlx::query_as::<_, CognitiveSpaceMemberDb>(
            r#"
            SELECT m.space_id, m.user_id, m.role, m.created_at
            FROM cognitive_space_members m
            WHERE m.space_id = $1
              AND EXISTS (
                SELECT 1
                FROM cognitive_space_members current_member
                WHERE current_member.space_id = m.space_id
                  AND current_member.user_id = $2
              )
            ORDER BY
                CASE m.role
                    WHEN 'owner' THEN 0
                    WHEN 'editor' THEN 1
                    ELSE 2
                END,
                m.created_at ASC
            "#,
        )
        .bind(space_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn update_member_role(
        &self,
        space_id: Uuid,
        user_id: Uuid,
        role: SpaceMemberRole,
    ) -> Result<Option<CognitiveSpaceMemberDb>, Error> {
        sqlx::query_as::<_, CognitiveSpaceMemberDb>(
            r#"
            UPDATE cognitive_space_members
            SET role = $3
            WHERE space_id = $1 AND user_id = $2 AND role <> 'owner'
            RETURNING space_id, user_id, role, created_at
            "#,
        )
        .bind(space_id)
        .bind(user_id)
        .bind(role.to_string())
        .fetch_optional(&self.pool)
        .await
    }

    async fn create_invite(
        &self,
        invite: CreateSpaceInvite,
    ) -> Result<CognitiveSpaceInviteDb, Error> {
        let expires_at = invite
            .expires_in_days
            .map(|days| Utc::now() + Duration::days(days));

        sqlx::query_as::<_, CognitiveSpaceInviteDb>(
            r#"
            INSERT INTO cognitive_space_invites
                (space_id, code, role, created_by, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(invite.space_id)
        .bind(invite.code)
        .bind(invite.role.to_string())
        .bind(invite.created_by)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
    }

    async fn accept_invite(
        &self,
        code: &str,
        user_id: Uuid,
    ) -> Result<Option<CognitiveSpaceInviteDb>, Error> {
        let mut tx = self.pool.begin().await?;

        let invite = sqlx::query_as::<_, CognitiveSpaceInviteDb>(
            r#"
            SELECT *
            FROM cognitive_space_invites
            WHERE code = $1
              AND accepted_at IS NULL
              AND (expires_at IS NULL OR expires_at > NOW())
            FOR UPDATE
            "#,
        )
        .bind(code)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(invite) = invite else {
            tx.commit().await?;
            return Ok(None);
        };

        sqlx::query(
            r#"
            INSERT INTO cognitive_space_members (space_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (space_id, user_id)
            DO UPDATE SET role = CASE
                WHEN cognitive_space_members.role = 'owner' THEN cognitive_space_members.role
                ELSE EXCLUDED.role
            END
            "#,
        )
        .bind(invite.space_id)
        .bind(user_id)
        .bind(&invite.role)
        .execute(&mut *tx)
        .await?;

        let accepted = sqlx::query_as::<_, CognitiveSpaceInviteDb>(
            r#"
            UPDATE cognitive_space_invites
            SET accepted_by = $2, accepted_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(invite.id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(accepted))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_cognitive_space_keeps_owner_boundary() {
        let owner_user_id = Uuid::new_v4();
        let space = CreateCognitiveSpace {
            name: "Personal Space".to_string(),
            description: Some("Private cognitive space".to_string()),
            owner_user_id,
            space_type: CognitiveSpaceType::Personal,
        };

        assert_eq!(space.owner_user_id, owner_user_id);
        assert_eq!(space.name, "Personal Space");
    }

    #[test]
    fn role_permissions_are_conservative() {
        assert!(SpaceMemberRole::Owner.can_write());
        assert!(SpaceMemberRole::Owner.can_manage_members());
        assert!(SpaceMemberRole::Editor.can_write());
        assert!(!SpaceMemberRole::Editor.can_manage_members());
        assert!(!SpaceMemberRole::Viewer.can_write());
        assert!(!SpaceMemberRole::Viewer.can_manage_members());
        assert!(!SpaceMemberRole::Owner.is_invitable());
        assert!(SpaceMemberRole::Viewer.is_invitable());
    }
}

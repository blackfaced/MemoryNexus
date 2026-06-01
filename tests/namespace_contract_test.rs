use uuid::Uuid;

use memorynexus::db::namespace::{CreateNamespace, NamespaceKind, NamespaceStatus};

#[test]
fn namespace_migration_scopes_names_inside_spaces() {
    let sql = std::fs::read_to_string("migrations/009_namespaces.sql").unwrap();

    assert!(sql.contains("CREATE TABLE IF NOT EXISTS namespaces"));
    assert!(
        sql.contains("space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE")
    );
    assert!(sql.contains("UNIQUE (space_id, name)"));
    assert!(sql.contains("UNIQUE (id, space_id)"));
    assert!(sql.contains("CHECK (kind IN ('reflective', 'skill'))"));
    assert!(sql.contains("status"));
}

#[test]
fn namespace_kind_accepts_only_reflective_or_skill() {
    assert_eq!(
        serde_json::from_str::<NamespaceKind>("\"reflective\"").unwrap(),
        NamespaceKind::Reflective
    );
    assert_eq!(
        serde_json::from_str::<NamespaceKind>("\"skill\"").unwrap(),
        NamespaceKind::Skill
    );
    assert!(serde_json::from_str::<NamespaceKind>("\"project\"").is_err());
}

#[test]
fn create_namespace_keeps_space_scope_without_permission_state() {
    let space_id = Uuid::new_v4();
    let created_by = Uuid::new_v4();
    let namespace = CreateNamespace {
        space_id,
        name: "learning.math".to_string(),
        kind: NamespaceKind::Skill,
        description: Some("Math practice feedback loop".to_string()),
        status: NamespaceStatus::Active,
        created_by,
    };

    assert_eq!(namespace.space_id, space_id);
    assert_eq!(namespace.created_by, created_by);
    assert_eq!(namespace.name, "learning.math");
    assert_eq!(namespace.kind, NamespaceKind::Skill);
}

#[test]
fn namespace_api_exposes_create_list_and_get_routes() {
    let source = std::fs::read_to_string("src/api/mod.rs").unwrap();

    assert!(source.contains("\"/api/v1/namespaces\""));
    assert!(source.contains("axum::routing::post(namespaces::create)"));
    assert!(source.contains("axum::routing::get(namespaces::list)"));
    assert!(source.contains("\"/api/v1/namespaces/:id\""));
    assert!(source.contains("axum::routing::get(namespaces::get)"));
}

#[test]
fn namespace_repository_uses_space_membership_for_visibility() {
    let source = std::fs::read_to_string("src/db/namespace.rs").unwrap();

    assert!(source.contains("INNER JOIN cognitive_space_members m ON m.space_id = n.space_id"));
    assert!(source.contains("WHERE n.space_id = $1 AND m.user_id = $2"));
    assert!(source.contains("WHERE n.id = $1 AND m.user_id = $2"));
}

#[test]
fn namespace_api_checks_space_access_before_create_and_list() {
    let source = std::fs::read_to_string("src/api/namespaces.rs").unwrap();

    assert!(source.contains("find_for_user(req.space_id, auth_user.user_id)"));
    assert!(source.contains("find_for_user(query.space_id, auth_user.user_id)"));
    assert!(source.contains("ok_or(AppError::Unauthorized)"));
}

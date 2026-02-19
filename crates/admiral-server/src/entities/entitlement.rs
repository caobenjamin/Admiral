use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "entitlements")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub invite_id: Option<Uuid>,
    pub r#type: String,
    pub starts_at: DateTimeWithTimeZone,
    pub expires_at: DateTimeWithTimeZone,
    pub is_active: bool,
    pub created_at: DateTimeWithTimeZone,
    pub extended_by: Option<Uuid>,
    pub extend_reason: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::invite::Entity",
        from = "Column::InviteId",
        to = "super::invite::Column::Id"
    )]
    Invite,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

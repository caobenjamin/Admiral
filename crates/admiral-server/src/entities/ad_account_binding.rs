use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "ad_account_bindings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub platform: String,
    pub account_id: String,
    pub oauth_token_id: Uuid,
    pub account_name: Option<String>,
    /// 绑定时所选组织，用于后续 MAPI 等查询
    pub subject_type: Option<i32>,
    pub subject_id: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
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
        belongs_to = "super::oauth_token::Entity",
        from = "Column::OauthTokenId",
        to = "super::oauth_token::Column::Id"
    )]
    OauthToken,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::oauth_token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OauthToken.def()
    }
}

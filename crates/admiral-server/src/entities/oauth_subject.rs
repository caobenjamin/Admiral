use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "oauth_subjects")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub oauth_token_id: Uuid,
    pub subject_type: i32,
    pub subject_id: i32,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::oauth_token::Entity",
        from = "Column::OauthTokenId",
        to = "super::oauth_token::Column::Id"
    )]
    OauthToken,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::oauth_token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OauthToken.def()
    }
}

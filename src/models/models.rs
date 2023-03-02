use crate::models::itunes_models::{Podcast, PodcastEpisode};
use diesel::prelude::*;
use diesel::sql_types::{Integer, Text};
// decode request data
#[derive(Deserialize)]
pub struct UserData {
    pub username: String,
}
// this is to insert users to database
#[derive(Serialize, Deserialize)]
pub struct NewUser {
    pub username: String,
    pub password: String,
    pub first_name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PodCastAddModel {
    pub track_id: i32,
    pub user_id: i32
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PodcastWatchedPostModel {
    pub podcast_episode_id: String,
    pub time: i32
}

#[derive(Serialize, Deserialize, Queryable, QueryableByName, Clone )]
#[serde(rename_all = "camelCase")]
pub struct PodcastHistoryItem {
    #[sql_type = "Integer"]
    pub id : i32,
    #[sql_type = "Integer"]
    pub podcast_id: i32,
    #[sql_type = "Text"]
    pub episode_id: String,
    #[sql_type = "Integer"]
    pub watched_time: i32,
    #[sql_type = "Text"]
    pub date: String
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PodcastWatchedEpisodeModel {
    pub id : i32,
    pub podcast_id: i32,
    pub episode_id: String,
    pub url: String,
    pub name: String,
    pub image_url: String,
    pub watched_time: i32,
    pub date: String,
    pub total_time: i32
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PodcastWatchedEpisodeModelWithPodcastEpisode {
    pub id : i32,
    pub podcast_id: i32,
    pub episode_id: String,
    pub url: String,
    pub name: String,
    pub image_url: String,
    pub watched_time: i32,
    pub date: String,
    pub total_time: i32,
    pub podcast_episode: PodcastEpisode,
    pub podcast: Podcast,
}
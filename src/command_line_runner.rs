use crate::config::dbconfig::establish_connection;
use crate::constants::inner_constants::{Role, ENVIRONMENT_SERVICE};
use crate::controllers::sys_info_controller::built_info;
use crate::models::episode::Episode;
use crate::models::favorites::Favorite;
use crate::models::podcasts::Podcast;
use crate::models::session::Session;
use crate::models::subscription::Subscription;
use crate::models::user::{User, UserWithoutPassword};
use crate::service::podcast_episode_service::PodcastEpisodeService;
use crate::service::rust_service::PodcastService;
use crate::utils::error::CustomError;
use crate::utils::time::get_current_timestamp_str;
use log::error;
use rpassword::read_password;
use sha256::digest;
use std::env::Args;
use std::io::{stdin, stdout, Error, ErrorKind, Write};
use std::process::exit;
use std::str::FromStr;
use crate::application::services::device::service::DeviceService;
use crate::application::usecases::devices::edit_use_case::EditUseCase;
use crate::dbconfig::DBType;
use crate::{init_postgres_db_pool, init_sqlite_db_pool};

pub async fn start_command_line(mut args: Args) {

    let pool;
    {
        let conn = establish_connection();
        match conn {
            DBType::Postgresql(_) => {
                pool = init_postgres_db_pool(&ENVIRONMENT_SERVICE.get().unwrap().database_url)
                    .await
                    .expect("Failed to connect to database");
            }
            DBType::Sqlite(_) => {
                pool = init_sqlite_db_pool(&ENVIRONMENT_SERVICE.get().unwrap().database_url)
                    .await
                    .expect("Failed to connect to database");
            }
        }
    }
    println!("Starting from command line");
    // This needs to be nth(1) because the first argument is the binary name
    match args.nth(1).unwrap().as_str() {
        "help" | "--help" => {
            println!(
                r" The following commands are available:
            users => Handles user management
            podcasts => Handles podcast management
            "
            )
        }
        "podcasts" => {
            println!("Podcast management");
            match args.next().unwrap().as_str() {
                "refresh" => {
                    let podcast_rss_feed = args.next();

                    match podcast_rss_feed {
                        Some(feed) => {
                            let mut podcast_service = PodcastService::new();
                            let conn = &mut establish_connection();

                            let replaced_feed = feed.replace(['\'', ' '], "");
                            println!("Refreshing podcast {}", replaced_feed);

                            let podcast = Podcast::get_podcast_by_rss_feed(replaced_feed, conn)
                                .expect("Error getting podcast");

                            PodcastEpisodeService::insert_podcast_episodes(conn, podcast.clone())
                                .unwrap();
                            podcast_service
                                .schedule_episode_download(podcast, None, conn)
                                .unwrap();
                        }
                        None => {
                            println!("Please provide a podcast rss feed url");
                            exit(1);
                        }
                    }
                }
                "refresh-all" => {
                    let conn = &mut establish_connection();
                    let podcasts = Podcast::get_all_podcasts(&mut establish_connection());
                    let mut podcast_service = PodcastService::new();
                    for podcast in podcasts.unwrap() {
                        println!("Refreshing podcast {}", podcast.name);

                        PodcastEpisodeService::insert_podcast_episodes(
                            &mut establish_connection(),
                            podcast.clone(),
                        )
                        .unwrap();
                        podcast_service
                            .schedule_episode_download(podcast, None, conn)
                            .unwrap();
                    }
                }
                "list" => {
                    let podcasts = Podcast::get_all_podcasts(&mut establish_connection());
                    match podcasts {
                        Ok(podcasts) => {
                            println!("Id - Name - RSS Feed");
                            for podcast in podcasts {
                                println!("{} - {} - {}", podcast.id, podcast.name, podcast.rssfeed);
                            }
                        }
                        Err(..) => {
                            println!("Error getting podcasts");
                        }
                    }
                }
                "help" | "--help" => {
                    println!(
                        r" The following commands are available:
                    refresh => Refreshes a podcast
                    refresh-all => Refreshes all podcasts
                    list => Lists all podcasts
                    "
                    )
                }
                _ => {
                    println!("Unknown command");
                }
            }
        }
        "users" => {
            println!("User management");
            match args.next().unwrap().as_str() {
                "add" => {
                    let mut user = read_user_account().unwrap();

                    println!(
                        "Should a user with the following settings be applied {:?}",
                        user
                    );

                    if ask_for_confirmation().is_ok() {
                        user.password = Some(digest(user.password.unwrap()));
                        if User::insert_user(&mut user, &mut establish_connection()).is_ok() {
                            println!("User succesfully created")
                        }
                    }
                }
                "generate" => match args.next().unwrap().as_str() {
                    "apiKey" => {
                        let conn = &mut establish_connection();
                        User::find_all_users(conn).iter().for_each(|u| {
                            log::info!("Updating api key of user {}", &u.username);
                            User::update_api_key_of_user(
                                &u.username,
                                uuid::Uuid::new_v4().to_string(),
                                conn,
                            )
                            .expect("Error updating api key");
                        })
                    }
                    _ => {
                        error!("Command not found")
                    }
                },
                "remove" => {
                    let mut username = String::new();
                    // remove user
                    let available_users = list_users();
                    retry_read(
                        "Please enter the username of the user you want to delete",
                        &mut username,
                    );
                    username = trim_string(&username);
                    match available_users.iter().find(|u| u.username == username) {
                        Some(..) => {
                            Episode::delete_by_username(&mut establish_connection(), &username)
                                .expect("Error deleting entries for podcast history item");
                            DeviceService::delete_by_username(&username, &pool)
                                .expect("Error deleting devices");
                            Episode::delete_by_username_and_episode(
                                &username,
                                &mut establish_connection(),
                            )
                            .expect("Error deleting episodes");
                            Favorite::delete_by_username(
                                trim_string(&username),
                                &mut establish_connection(),
                            )
                            .expect("Error deleting favorites");
                            Session::delete_by_username(
                                &trim_string(&username),
                                &mut establish_connection(),
                            )
                            .expect("Error deleting sessions");
                            Subscription::delete_by_username(
                                &trim_string(&username),
                                &mut establish_connection(),
                            )
                            .expect("TODO: panic message");
                            User::delete_by_username(
                                trim_string(&username),
                                &mut establish_connection(),
                            )
                            .expect("Error deleting user");
                            println!("User deleted")
                        }
                        None => {
                            println!("Username not found")
                        }
                    }
                }
                "update" => {
                    //update a user
                    list_users();
                    let mut username = String::new();

                    retry_read(
                        "Please enter the username of the user you want to update",
                        &mut username,
                    );
                    username = trim_string(&username);
                    println!(">{}<", username);
                    let user =
                        User::find_by_username(username.as_str(), &mut establish_connection())
                            .unwrap();

                    do_user_update(user)
                }
                "list" => {
                    // list users

                    list_users();
                }
                "help" | "--help" => {
                    println!(
                        r" The following commands are available:
                    add => Adds a user
                    remove => Removes a user
                    update => Updates a user
                    list => Lists all users
                    "
                    )
                }
                _ => {
                    error!("Command not found")
                }
            }
        }
        "migration" => {
            error!("Command not found")
        }
        "debug" => {
            create_debug_message();
        }
        _ => {
            error!("Command not found")
        }
    }
}

fn list_users() -> Vec<UserWithoutPassword> {
    let users = User::find_all_users(&mut establish_connection());

    users.iter().for_each(|u| {
        println!("|Username|Role|Explicit Consent|Created at|",);
        println!(
            "|{}|{}|{}|{}|",
            u.username, u.role, u.explicit_consent, u.created_at
        );
    });
    users
}

pub fn read_user_account() -> Result<User, CustomError> {
    let mut username = String::new();

    let role = Role::VALUES.map(|v| v.to_string()).join(", ");
    retry_read("Enter your username: ", &mut username);

    let user = User::find_by_username(&username, &mut establish_connection());

    if user.is_err() {
        println!("User does not exist");
    }

    let password = retry_read_secret("Enter your password: ");
    let assigned_role = retry_read_role(&format!("Select your role {}", &role));
    let mut api_key_generated = uuid::Uuid::new_v4().to_string();
    api_key_generated = api_key_generated.replace('-', "");

    let user = User {
        id: 0,
        username: trim_string(&username),
        role: assigned_role.to_string(),
        password: Some(trim_string(&password)),
        explicit_consent: false,
        created_at: get_current_timestamp_str(),
        api_key: Some(api_key_generated),
    };

    Ok(user)
}

pub fn retry_read(prompt: &str, input: &mut String) {
    println!("{}", prompt);
    stdin().read_line(input).unwrap();
    match !input.is_empty() {
        true => {
            if input.trim().is_empty() {
                retry_read(prompt, input);
            }
        }
        false => {
            retry_read(prompt, input);
        }
    }
}

pub fn retry_read_secret(prompt: &str) -> String {
    println!("{}", prompt);
    stdout().flush().unwrap();
    let input = read_password().unwrap();
    match !input.is_empty() {
        true => {
            if input.trim().is_empty() {
                retry_read_secret(prompt);
            }
        }
        false => {
            retry_read_secret(prompt);
        }
    }
    input
}

pub fn retry_read_role(prompt: &str) -> Role {
    let mut input = String::new();
    println!("{}", prompt);
    stdin().read_line(&mut input).unwrap();
    let res = Role::from_str(&trim_string(&input));
    match res {
        Err(..) => {
            println!("Error setting role. Please choose one of the possible roles.");
            retry_read_role(prompt)
        }
        Ok(..) => res.unwrap(),
    }
}

fn ask_for_confirmation() -> Result<(), Error> {
    let mut input = String::new();
    println!("Y[es]/N[o]");
    stdin()
        .read_line(&mut input)
        .expect("Error reading from terminal");
    match input.to_lowercase().starts_with('y') {
        true => Ok(()),
        false => Err(Error::new(ErrorKind::WouldBlock, "Interrupted by user.")),
    }
}

fn trim_string(string_to_trim: &str) -> String {
    string_to_trim
        .trim_end_matches('\n')
        .trim()
        .parse()
        .unwrap()
}

fn do_user_update(mut user: User) {
    let mut input = String::new();
    println!(
        "The following settings of a user should be updated: {:?}",
        user
    );
    println!(
        "Enter which field of a user should be updated [role, password, \
    consent]"
    );
    stdin()
        .read_line(&mut input)
        .expect("Error reading from terminal");
    input = trim_string(&input);
    match input.as_str() {
        "role" => {
            user.role = Role::to_string(&retry_read_role(
                "Enter the new role [user,\
            uploader or admin]",
            ));
            User::update_user(user, &mut establish_connection()).expect("Error updating role");
            println!("Role updated");
        }
        "password" => {
            let mut password = retry_read_secret("Enter the new password");
            password = digest(password);
            user.password = Some(password);
            User::update_user(user, &mut establish_connection()).expect("Error updating password");
            println!("Password updated");
        }
        "consent" => {
            user.explicit_consent = !user.explicit_consent;
            User::update_user(user, &mut establish_connection()).expect("Error switching consent");
            println!("Consent preference switched");
        }
        _ => {
            println!("Field not found");
        }
    }
}

pub fn create_debug_message() {
    println!("OS: {}", built_info::CFG_OS);
    println!("Target: {}", built_info::TARGET);
    println!("Endian: {}", built_info::CFG_ENDIAN);
    println!("Debug: {}", built_info::DEBUG);
    println!("Git Version: {:?}", "");
    println!("Git Commit Hash: {:?}", "");
    println!("Git Head Ref: {:?}", "");
    println!("Build Time: {}", built_info::BUILT_TIME_UTC);
    println!("Version: {}", built_info::PKG_VERSION);
    println!("Authors: {}", built_info::PKG_AUTHORS);
    println!("Name: {}", built_info::PKG_NAME);
    println!("Description: {}", built_info::PKG_DESCRIPTION);
    println!("Homepage: {}", built_info::PKG_HOMEPAGE);
    println!("Repository: {}", built_info::PKG_REPOSITORY);
    println!("Rustc Version: {}", built_info::RUSTC_VERSION);
    println!("Rustc: {}", built_info::RUSTC_VERSION);

    let podcasts = Podcast::get_all_podcasts(&mut establish_connection());

    match podcasts {
        Ok(podcasts) => {
            podcasts.iter().for_each(|p| {
                println!("Podcast: {:?}", p);
            });
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}

//! CLI commands - subcommands for the CLI.

pub mod agent;
pub mod channel;
pub mod cron;
pub mod discord_test;
pub mod gateway;

pub use agent::{execute as agent, interactive as agent_interactive};
pub use channel::{login as channel_login, status as channel_status};
pub use cron::{add as cron_add, enable as cron_enable, list as cron_list, remove as cron_remove};
pub use discord_test::execute as discord_test;
pub use gateway::execute as gateway;

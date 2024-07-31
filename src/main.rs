use clap::{Parser, Subcommand};
use libc::getpwnam;
use std::{
    ffi::CString,
    fs::File,
    io::{BufRead, BufReader, Error, ErrorKind, Result},
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Entry {
        #[arg(short = 'u', long)]
        users_file: PathBuf,
    },
    CreateUser {
        #[arg(short = 'n', long)]
        user_name: String,

        #[arg(short = 'u', long)]
        user_id: u32,

        #[arg(short = 'g', long)]
        user_group_id: u32,

        #[arg(short = 'p', long)]
        user_pass: String,
    },
    DeleteUser {
        #[arg(short = 'n', long)]
        user_name: String,

        #[arg(short = 'u', long)]
        user_id: u32,

        #[arg(short = 'g', long)]
        user_group_id: u32,
    },
    ModifyUser {
        #[arg(short = 'n', long)]
        user_old_name: String,

        #[arg(short = 'u', long)]
        user_old_id: u32,

        #[arg(short = 'g', long)]
        user_old_group_id: u32,

        #[arg(short = 'N', long)]
        user_new_name: Option<String>,

        #[arg(short = 'U', long)]
        user_new_id: Option<u32>,

        #[arg(short = 'G', long)]
        user_new_group_id: Option<u32>,

        #[arg(short = 'P', long)]
        user_new_pass: Option<String>,
    },
}

fn sftp_entry(users: PathBuf) -> Result<()> {
    let users_path = users.clone().into_boxed_path();
    if !users_path.exists() {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!("Path {users_path:?} does not exist"),
        ));
    }

    Ok(())
}

fn sftp_create_user(
    user_name: String,
    user_id: u32,
    user_group_id: u32,
    user_pass: String,
) -> Result<()> {
    let c_user_name = CString::new(user_name.clone()).expect("String conversion failed");
    let c_user_pw = unsafe { getpwnam(c_user_name.as_ptr()) };
    if !c_user_pw.is_null() {
        return Err(Error::new(
            ErrorKind::AlreadyExists,
            format!("User {user_name} already exists"),
        ));
    }

    let result = Command::new("useradd")
        .arg("-m")
        .arg("-u")
        .arg(user_id.clone().to_string())
        .arg("-g")
        .arg(user_group_id.clone().to_string())
        .arg("-d")
        .arg(format!("/home/{user_name}"))
        .arg("-p")
        .arg(user_pass.clone().to_string())
        .arg(user_name.clone().to_string())
        .output()?;

    if result.status.success() {
        println!(
            "User {} (uid: {} gid: {}) created successfully!",
            user_name, user_id, user_group_id
        );
    } else {
        return Err(Error::new(
            ErrorKind::Other,
            String::from_utf8(result.stderr).expect("String conversion failed"),
        ));
    }

    Ok(())
}

fn sftp_delete_user(user_name: String, user_id: u32, user_group_id: u32) -> Result<()> {
    let c_user_name = CString::new(user_name.clone()).expect("String conversion failed");
    let c_user_pw = unsafe { getpwnam(c_user_name.as_ptr()) };
    if c_user_pw.is_null() {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!("User {user_name} doens't exist"),
        ));
    }

    let result = Command::new("userdel")
        .arg("-r")
        .arg(user_name.clone())
        .output()?;

    if result.status.success() {
        println!(
            "User {} (uid: {} gid: {}) deleted successfully!",
            user_name, user_id, user_group_id
        );
    } else {
        return Err(Error::new(
            ErrorKind::Other,
            String::from_utf8(result.stderr).expect("String conversion failed"),
        ));
    }

    Ok(())
}

fn sftp_modify_user(
    user_old_name: String,
    user_old_id: u32,
    user_old_group_id: u32,
    user_new_name: Option<String>,
    user_new_id: Option<u32>,
    user_new_group_id: Option<u32>,
    user_new_pass: Option<String>,
) -> Result<()> {
    let c_user_name = CString::new(user_old_name.clone()).expect("String conversion failed");
    let c_user_pw = unsafe { getpwnam(c_user_name.as_ptr()) };
    if c_user_pw.is_null() {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!("User {user_old_name} doens't exist"),
        ));
    }

    let mut command = Command::new("usermod");
    if let Some(ref user_new_name) = user_new_name {
        command.arg("-l").arg(user_new_name);
    }
    if let Some(ref user_new_id) = user_new_id {
        command.arg("-u").arg(user_new_id.to_string());
    }
    if let Some(ref user_new_group_id) = user_new_group_id {
        command.arg("-g").arg(user_new_group_id.to_string());
    }
    command.arg(user_old_name.clone());

    let result = command.output()?;

    if result.status.success() {
        let (user_name, user_id, user_group_id) = (
            user_new_name.unwrap_or(user_old_name),
            user_new_id.unwrap_or(user_old_id),
            user_new_group_id.unwrap_or(user_old_group_id),
        );
        println!(
            "User {} (uid: {} gid: {}) modified successfully!",
            user_name, user_id, user_group_id
        );

        if let Some(user_new_pass) = user_new_pass {
            let result = Command::new("chpasswd")
                .arg("-e")
                .arg(format!("{}:{}", user_name, user_new_pass))
                .output()?;

            if result.status.success() {
                println!(
                    "Password for user {} (uid: {} gid: {}) changed successfully!",
                    user_name, user_id, user_group_id
                );
            } else {
                return Err(Error::new(
                    ErrorKind::Other,
                    String::from_utf8(result.stderr).expect("String conversion failed"),
                ));
            }
        }
    } else {
        return Err(Error::new(
            ErrorKind::Other,
            String::from_utf8(result.stderr).expect("String conversion failed"),
        ));
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Entry { users_file } => sftp_entry(users_file),
        Commands::CreateUser {
            user_name,
            user_id,
            user_group_id,
            user_pass,
        } => sftp_create_user(user_name, user_id, user_group_id, user_pass),
        Commands::DeleteUser {
            user_name,
            user_id,
            user_group_id,
        } => sftp_delete_user(user_name, user_id, user_group_id),
        Commands::ModifyUser {
            user_old_name,
            user_old_id,
            user_old_group_id,
            user_new_name,
            user_new_id,
            user_new_group_id,
            user_new_pass,
        } => sftp_modify_user(
            user_old_name,
            user_old_id,
            user_old_group_id,
            user_new_name,
            user_new_id,
            user_new_group_id,
            user_new_pass,
        ),
    }
}

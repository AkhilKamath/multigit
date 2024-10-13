use std::collections::HashMap;
use std::env::var;
use std::fs;
use std::path::PathBuf;
use std::io::{Error, Result};
use std::process::Command;


struct GitAccount {
    name: String,
    email: String,
    ssh_key: String,
}

struct GitAccountManager {
    accounts: HashMap<String, GitAccount>,
    config_path: PathBuf,
}

impl GitAccountManager {
    fn new(config_path: PathBuf) -> Self {
        GitAccountManager {
            accounts: HashMap::new(),
            config_path,
        }
    }

    fn add_account(&mut self, name: &str, email: &str, ssh_key: &str) {
        let account = GitAccount { 
            name: name.to_string(),
            email: email.to_string(),
            ssh_key: ssh_key.to_string(),
        };
        self.accounts.insert(name.to_string(), account);
    }

    fn generate_ssh_key(&mut self, account_name: &str, email: &str) -> Result<String> {
        let ssh_dir = self.config_path.join(".ssh");
        fs::create_dir_all(&ssh_dir)?;

        let key_file = ssh_dir.join(format!("id_ed25519_{}", account_name));
        let key_file_str = key_file.to_str().unwrap();

        let output = Command::new("ssh-keygen")
            .args([
                "-t", "ed25519",
                "-C", email,
                "-f", key_file_str,
                "-N", "", // TODO: add passphrase support
            ])
            .output()?;

        if !output.status.success() {
            return Err(Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to generate SSH key: {}", String::from_utf8_lossy(&output.stderr))
            ));
        }

        Ok(key_file_str.to_string())
    }

    fn setup_account(&self, name: &str, email: &str, dir: &str) -> Result<()> {
        let ssh_key =self.generate_ssh_key(name, email)?;
        self.add_account(name, email, &ssh_key);

        Ok(())
    }

    fn associate_account_with_dir(&self, account_name: &str, dir: &str) -> Result<()> {
        let account = self.accounts.get(account_name).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Account not found")
        })?;
        let config_content = format!(
            "[url \"git@github.com-{}:\"]\n    insteadOf = git@github.com:\n[user]\n    name = {}\n    email = {}\"",
            account.name, account.name, account.email
        );

        let gitconfig_path = match account_name {
            "acc1" => self.config_path.join("Code/acc1/.gitconfig"),
            "acc2" => self.config_path.join("Code/acc2/.gitconfig"),
            _ => return Err(Error::new(std::io::ErrorKind::NotFound, "Account not found")),
        };

        fs::write(&gitconfig_path, config_content)?;

        // Create or append to the global .gitconfig
        let global_gitconfig_path = self.config_path.join(".gitconfig");
        let include_if_content = format!(
            "[includeIf \"gitdir:{}\"]\n    path = {}\n",
            dir,
            gitconfig_path.to_str().unwrap()
        );

        let mut global_gitconfig = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&global_gitconfig_path)?;

        global_gitconfig.write_all(include_if_content.as_bytes())?;


        Ok(())
    }
}

fn main() -> Result<()> {

    let home_dir = var("HOME").expect("$HOME directory not found");
    println!("Home directory: {}", home_dir);

    let config_path = PathBuf::from(home_dir);

    Ok(())
}
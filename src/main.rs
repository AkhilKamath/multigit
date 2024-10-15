use std::collections::HashMap;
use std::env::var;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::io::{Error, Read, Result, Write};
use std::process::Command;


struct GitAccount {
    name: String,
    email: String,
    ssh_key: String,
    codebase_dir_path: PathBuf,
}

struct GitAccountManager {
    accounts: HashMap<String, GitAccount>,
    home_dir: PathBuf,
}

impl GitAccountManager {
    fn new(home_dir: PathBuf) -> Self {
        GitAccountManager {
            accounts: HashMap::new(),
            home_dir,
        }
    }

    fn add_account(&mut self, name: &str, email: &str, ssh_key: &str, codebase_dir_path: PathBuf) {
        let account = GitAccount { 
            name: name.to_string(),
            email: email.to_string(),
            ssh_key: ssh_key.to_string(),
            codebase_dir_path,
        };
        self.accounts.insert(name.to_string(), account);
    }

    fn generate_ssh_key(&mut self, account_name: &str, email: &str) -> Result<String> {
        let ssh_dir = self.home_dir.join(".ssh");
        fs::create_dir_all(&ssh_dir).unwrap();

        let key_file = ssh_dir.join(format!("id_ed25519_{}", account_name));
        let key_file_str = key_file.to_str().unwrap();

        let output = Command::new("ssh-keygen")
            .args([
                "-t", "ed25519",
                "-C", email,
                "-f", key_file_str,
                "-N", "", // TODO: add passphrase support
            ])
            .output().unwrap();

        if !output.status.success() {
            return Err(Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to generate SSH key: {}, {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr))
            ));
        }
        println!("Key file: {}", key_file_str);
        Ok(key_file_str.to_string())
    }

    fn add_ssh_agent(&self, ssh_key_file: &str) -> Result<()> {
        let output = Command::new("ssh-add")
        .args([
            ssh_key_file
        ])
        .output().unwrap();

        if !output.status.success() {
            return Err(Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to add SSH key to agent: {}, {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr))
            ));
        }

        Ok(())
    }
    
    fn setup_local_gitconfig(&self, account: &GitAccount) -> Result<()> {
        let config_content = format!(
            "[url \"git@github.com-{}:\"]\n    insteadOf = git@github.com:\n[user]\n    name = {}\n    email = {}\"\n",
            account.name, account.name, account.email
        );

        let gitconfig_path = account.codebase_dir_path.join(".gitconfig");
        
        
        if let Some(parent) = gitconfig_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        
        println!("gitconfig path: {}", gitconfig_path.to_str().unwrap());
        println!("config content: {}", config_content.to_string());
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&gitconfig_path).unwrap();

        file.write_all(config_content.as_bytes()).unwrap();

        Ok(())
    }

    fn setup_global_gitconfig(&self, codebase_path_str: &str, global_gitconfig_path_str: &str) -> Result<()> {
        // Create or append to the global .gitconfig
        let global_gitconfig_path = self.home_dir.join(".gitconfig");
        let include_if_content = format!(
            "[includeIf \"gitdir/i:{}\"]\n    path = {}\n\n",
            codebase_path_str,
            global_gitconfig_path_str
        );

        let mut global_gitconfig = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&global_gitconfig_path).unwrap();

        println!("global gitconfig path: {}", global_gitconfig_path.to_str().unwrap());
        println!("includeIf content: {}", include_if_content.to_string());

        global_gitconfig.write_all(include_if_content.as_bytes()).unwrap();

        Ok(())
    }

    fn associate_account_with_dir(&mut self, account_name: &str) -> Result<()> {
        println!("Account name: {}", account_name);
        let account = self.accounts.get(account_name).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Account not found")
        }).unwrap();

        let codebase_path = &account.codebase_dir_path;

        let gitconfig_path = codebase_path.join(".gitconfig");

        let _ = self.setup_local_gitconfig(account);

        let codebase_path = codebase_path.to_str();

        let gitconfig_path = gitconfig_path.to_str();

        if let (Some(cb_path), Some(gc_path)) = (codebase_path, gitconfig_path) {
            let _ = self.setup_global_gitconfig(cb_path, gc_path);
        } else {
            eprintln!("One or both paths could not be converted to &str");
        }

        Ok(())
    }

    fn setup_ssh_config(&self, name: &str, host: &str) -> Result<()> {
        let ssh_config_path = self.home_dir.join(".ssh/config");

        let account = self.accounts.get(name).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Account not found")
        }).unwrap();

        let config_content = format!(
            "\nHost {}\n    HostName github.com\n    User git\n    AddKeysToAgent yes\n    UseKeychain yes\n    IdentityFile {}\n",
            host, account.ssh_key
        );

        // Read existing config
        let mut existing_config = String::new();

        if ssh_config_path.exists() {
            let mut file = fs::File::open(&ssh_config_path).unwrap();
            file.read_to_string(&mut existing_config).unwrap();
        }
        
        // Check if the configuration already exists
        if !existing_config.contains(&format!("Host {}", host)) {
            let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&ssh_config_path).unwrap();

            file.write_all(config_content.as_bytes()).unwrap();
        }

        Ok(())

    }

    fn setup_account(&mut self, name: &str, email: &str, codebase_dir: &str, host: &str) -> Result<()> {
        let ssh_key =self.generate_ssh_key(name, email).unwrap();
        self.add_ssh_agent(&ssh_key).unwrap();
        self.add_account(name, email, &ssh_key, self.home_dir.join(codebase_dir));
        let _ = self.associate_account_with_dir(name);
        let _ = self.setup_ssh_config(name, host);
        Ok(())
    }

}

fn main() -> Result<()> {

    let home_dir = var("HOME").expect("$HOME directory not found");
    println!("Home directory: {}", home_dir);

    let config_path = PathBuf::from(home_dir);

    let mut account_manager = GitAccountManager::new(config_path);

    let _ = account_manager.setup_account("AkhilKamath", "akhilkamath97@gmail.com", "/Users/akhil/Code/acc1/", "github.com-pers");

    Ok(())
}
use std::{
    env, fs,
    io::Write,
    path::PathBuf,
    process::{Command as ProcessCommand, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use turso::{Connection, params};

use crate::{
    Result,
    cli::{AddArgs, FilterArgs},
};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    command TEXT NOT NULL,
    executed_at INTEGER NOT NULL,
    cwd TEXT NOT NULL,
    shell TEXT NOT NULL,
    hostname TEXT NOT NULL,
    exit_status INTEGER,
    duration_ms INTEGER
);
CREATE INDEX IF NOT EXISTS history_executed_at ON history(executed_at DESC);
CREATE INDEX IF NOT EXISTS history_cwd_executed_at ON history(cwd, executed_at DESC);
";

const HIDE_INTERNAL: &str = "
AND command NOT LIKE '%commandline edit%'
AND command NOT LIKE '%terminal-history add%'
AND command NOT LIKE '%terminal-history pick%'
AND command NOT LIKE '%terminal-history recall%'";

struct Db {
    conn: Connection,
    remote: Option<turso::sync::Database>,
}

impl Db {
    async fn open(pull: bool) -> Result<Self> {
        let path = database_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let path = path.to_string_lossy();

        let (conn, remote) = if let Ok(url) = env::var("TURSO_DATABASE_URL") {
            let mut builder = turso::sync::Builder::new_remote(&path).with_remote_url(url);
            if let Ok(token) = env::var("TURSO_AUTH_TOKEN") {
                builder = builder.with_auth_token(token);
            }
            let db = builder.build().await?;
            if pull {
                db.pull().await?;
            }
            let conn = db.connect().await?;
            (conn, Some(db))
        } else {
            let db = turso::Builder::new_local(&path).build().await?;
            (db.connect()?, None)
        };

        conn.execute_batch(SCHEMA).await?;
        if let Some(db) = &remote {
            db.push().await?;
        }
        Ok(Self { conn, remote })
    }

    async fn push(&self) -> Result<()> {
        if let Some(db) = &self.remote {
            db.push().await?;
        }
        Ok(())
    }
}

pub async fn add(args: AddArgs) -> Result<()> {
    if args.command.trim().is_empty() || is_internal_command(&args.command) {
        return Ok(());
    }
    let cwd = args.cwd.unwrap_or(pwd()?);
    let hostname = args
        .hostname
        .or_else(|| env::var("HOSTNAME").ok())
        .unwrap_or_default();
    let timestamp = args
        .timestamp
        .unwrap_or(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64);

    let db = Db::open(false).await?;
    db.conn
        .execute(
            "INSERT INTO history
             (command, executed_at, cwd, shell, hostname, exit_status, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                args.command,
                timestamp,
                cwd.to_string_lossy(),
                args.shell,
                hostname,
                args.status,
                args.duration
            ],
        )
        .await?;
    db.push().await
}

pub async fn list(filter: FilterArgs, query: Option<String>) -> Result<()> {
    let cwd = if filter.all {
        None
    } else {
        Some(filter.cwd.unwrap_or(pwd()?).to_string_lossy().into_owned())
    };
    let pattern = query.map(|value| format!("%{value}%"));
    let db = Db::open(true).await?;
    let sql = format!(
        "SELECT datetime(executed_at, 'unixepoch', 'localtime'), cwd, command,
                exit_status, duration_ms
         FROM history
         WHERE (?1 IS NULL OR command LIKE ?1)
           AND (?2 IS NULL OR cwd = ?2)
           {HIDE_INTERNAL}
         ORDER BY executed_at DESC, id DESC LIMIT ?3"
    );
    let mut rows = db
        .conn
        .query(&sql, params![pattern, cwd, filter.limit])
        .await?;

    while let Some(row) = rows.next().await? {
        let time: String = row.get(0)?;
        let cwd: String = row.get(1)?;
        let command: String = row.get(2)?;
        let status: Option<i64> = row.get(3)?;
        let duration: Option<i64> = row.get(4)?;
        println!(
            "{time}\t{}\t{}\t{cwd}\t{command}",
            status.map_or_else(|| "-".into(), |v| v.to_string()),
            duration.map_or_else(|| "-".into(), |v| format!("{v}ms")),
        );
    }
    Ok(())
}

pub async fn recall(prefix: &str, offset: i64) -> Result<()> {
    let db = Db::open(false).await?;
    let sql = format!(
        "SELECT command FROM history
         WHERE cwd = ?1 AND command LIKE ?2 ESCAPE '\\'
           {HIDE_INTERNAL}
         GROUP BY command ORDER BY MAX(executed_at) DESC, MAX(id) DESC
         LIMIT 1 OFFSET ?3"
    );
    let mut rows = db
        .conn
        .query(
            &sql,
            params![
                pwd()?.to_string_lossy(),
                format!("{}%", escape_like(prefix)),
                offset
            ],
        )
        .await?;
    if let Some(row) = rows.next().await? {
        print!("{}", row.get::<String>(0)?);
    }
    Ok(())
}

pub async fn pick(query: &str) -> Result<()> {
    let db = Db::open(true).await?;
    let sql = format!(
        "SELECT command FROM history
         WHERE cwd = ?1 AND command LIKE ?2 ESCAPE '\\'
           {HIDE_INTERNAL}
         GROUP BY command ORDER BY MAX(executed_at) DESC, MAX(id) DESC
         LIMIT 1000"
    );
    let mut rows = db
        .conn
        .query(
            &sql,
            params![
                pwd()?.to_string_lossy(),
                format!("%{}%", escape_like(query))
            ],
        )
        .await?;
    let mut commands = Vec::new();
    while let Some(row) = rows.next().await? {
        commands.push(row.get::<String>(0)?);
    }
    let Some(first) = commands.first() else {
        return Ok(());
    };

    let selector = env::var("TERMINAL_HISTORY_SELECTOR").unwrap_or_else(|_| "fzf".into());
    let Ok(mut child) = ProcessCommand::new(selector)
        .args([
            "--read0",
            "--print0",
            "--height=40%",
            "--reverse",
            "--query",
            query,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    else {
        print!("{first}");
        return Ok(());
    };
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or("selector stdin is unavailable")?;
        for command in &commands {
            stdin.write_all(command.as_bytes())?;
            stdin.write_all(&[0])?;
        }
    }
    let output = child.wait_with_output()?;
    if output.status.success() {
        print!(
            "{}",
            String::from_utf8_lossy(&output.stdout).trim_end_matches('\0')
        );
    }
    Ok(())
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn is_internal_command(command: &str) -> bool {
    if [
        "commandline edit",
        "terminal-history add",
        "terminal-history pick",
        "terminal-history recall",
    ]
    .iter()
    .any(|internal| command.contains(internal))
    {
        return true;
    }
    let executable = command
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_start_matches('^');
    PathBuf::from(executable)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, "terminal-history" | "commandline"))
}

fn database_path() -> Result<PathBuf> {
    if let Some(path) = env::var_os("HISTORY_DATABASE_PATH") {
        return Ok(path.into());
    }
    let home = env::var_os("HOME").ok_or("HOME is not set; set HISTORY_DATABASE_PATH")?;
    Ok(PathBuf::from(home).join(".local/share/terminal-history/history.db"))
}

fn pwd() -> Result<PathBuf> {
    Ok(env::var_os("PWD")
        .map(PathBuf::from)
        .unwrap_or(env::current_dir()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_sql_like_prefix() {
        assert_eq!(escape_like(r"50%_done\"), r"50\%\_done\\");
    }

    #[test]
    fn hides_internal_commands() {
        assert!(is_internal_command("commandline edit foo"));
        assert!(is_internal_command(
            "^/tmp/terminal-history recall --prefix git"
        ));
        assert!(is_internal_command(
            "let selected = (^/tmp/terminal-history recall --prefix git)"
        ));
        assert!(!is_internal_command("cargo test"));
    }
}

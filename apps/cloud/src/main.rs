use amanclaw_cloud::{api, db::CloudDb, invite, router::TenantRouter, state::CloudState, tenant};
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "amanclaw-cloud", version, about = "AmanClaw Cloud — managed hosting")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the cloud server
    Serve {
        #[arg(short, long, default_value = "8443")]
        port: u16,
        #[arg(long, default_value = "cloud/cloud.db")]
        db_path: String,
    },
    /// Manage invite codes
    Invite {
        #[command(subcommand)]
        action: InviteAction,
    },
    /// Manage tenants
    Tenant {
        #[command(subcommand)]
        action: TenantAction,
    },
}

#[derive(Subcommand)]
enum InviteAction {
    /// Create a new invite code
    Create {
        #[arg(long)]
        email: String,
        #[arg(long, default_value = "30")]
        days: i64,
    },
    /// List all invite codes
    List,
    /// Revoke an invite code
    Revoke { code: String },
}

#[derive(Subcommand)]
enum TenantAction {
    /// List all tenants
    List,
    /// Show tenant info
    Info { slug: String },
    /// Suspend a tenant
    Suspend { slug: String },
    /// Delete a tenant
    Delete { slug: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("amanclaw=info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port, db_path } => {
            // Ensure parent directory exists
            if let Some(parent) = std::path::Path::new(&db_path).parent() {
                std::fs::create_dir_all(parent).ok();
            }

            let db = CloudDb::new(&db_path).await?;
            let router = TenantRouter::new(db.clone());
            let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
                use rand::Rng;
                rand::rng()
                    .sample_iter(&rand::distr::Alphanumeric)
                    .take(64)
                    .map(char::from)
                    .collect()
            });

            let state = CloudState::new(db, router, jwt_secret);

            // Spawn idle cleanup task
            let cleanup_state = state.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(300)).await;
                    cleanup_state.router.write().await.cleanup_idle(1800).await;
                }
            });

            let app = api::cloud_router(state);
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
            tracing::info!(port, "AmanClaw Cloud server listening");
            axum::serve(listener, app).await?;
            Ok(())
        }
        Commands::Invite { action } => {
            let db = CloudDb::new("cloud/cloud.db").await?;
            match action {
                InviteAction::Create { email, days } => {
                    let inv = invite::create_invite(db.pool(), &email, days).await?;
                    println!("Invite created:");
                    println!("  Code:    {}", inv.code);
                    println!("  Email:   {}", inv.email);
                    println!("  Expires: {}", inv.expires_at);
                }
                InviteAction::List => {
                    let invites = invite::list_invites(db.pool()).await?;
                    if invites.is_empty() {
                        println!("No invites.");
                        return Ok(());
                    }
                    println!("{:<10} {:<30} {:<10} {:<25}", "Code", "Email", "Used", "Expires");
                    println!("{}", "-".repeat(77));
                    for inv in &invites {
                        let used = if inv.used_by.is_empty() { "no" } else { "yes" };
                        let expires = inv.expires_at.chars().take(19).collect::<String>();
                        println!("{:<10} {:<30} {:<10} {:<25}", inv.code, inv.email, used, expires);
                    }
                }
                InviteAction::Revoke { code } => {
                    if invite::revoke_invite(db.pool(), &code).await? {
                        println!("Invite {code} revoked.");
                    } else {
                        println!("Invite {code} not found.");
                    }
                }
            }
            Ok(())
        }
        Commands::Tenant { action } => {
            let db = CloudDb::new("cloud/cloud.db").await?;
            match action {
                TenantAction::List => {
                    let tenants = db.list_tenants().await?;
                    if tenants.is_empty() {
                        println!("No tenants.");
                        return Ok(());
                    }
                    println!("{:<20} {:<20} {:<10} {:<10}", "Slug", "Name", "Status", "Plan");
                    println!("{}", "-".repeat(62));
                    for t in &tenants {
                        println!("{:<20} {:<20} {:<10} {:<10}", t.slug, t.name, t.status, t.plan);
                    }
                }
                TenantAction::Info { slug } => {
                    match db.get_tenant_by_slug(&slug).await? {
                        Some(t) => {
                            println!("Tenant: {}", t.name);
                            println!("  Slug:    {}", t.slug);
                            println!("  Email:   {}", t.owner_email);
                            println!("  Status:  {}", t.status);
                            println!("  Plan:    {}", t.plan);
                            println!("  Created: {}", t.created_at);
                            println!("  Active:  {}", t.last_active);
                        }
                        None => println!("Tenant '{slug}' not found."),
                    }
                }
                TenantAction::Suspend { slug } => {
                    match db.get_tenant_by_slug(&slug).await? {
                        Some(t) => {
                            db.update_tenant_status(&t.id, "suspended").await?;
                            println!("Tenant '{slug}' suspended.");
                        }
                        None => println!("Tenant '{slug}' not found."),
                    }
                }
                TenantAction::Delete { slug } => {
                    match db.get_tenant_by_slug(&slug).await? {
                        Some(t) => {
                            db.update_tenant_status(&t.id, "deleted").await?;
                            tenant::deprovision_tenant(&t.id)?;
                            println!("Tenant '{slug}' deleted.");
                        }
                        None => println!("Tenant '{slug}' not found."),
                    }
                }
            }
            Ok(())
        }
    }
}

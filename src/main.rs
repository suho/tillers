use tillers::{Result, WorkspaceCreateRequest};
use tillers::services::{WorkspaceManager, WorkspaceManagerConfig};
use tokio;
use tracing::{info, error};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("tillers=debug,info")
        .init();

    info!("TilleRS - Keyboard-First Tiling Window Manager");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("Starting up...");

    // Initialize workspace manager
    let workspace_manager = WorkspaceManager::new(WorkspaceManagerConfig::default());
    info!("Workspace manager initialized");

    // Create a default workspace to demonstrate functionality
    let default_pattern_id = Uuid::new_v4(); // In real app, this would come from tiling patterns
    
    let create_request = WorkspaceCreateRequest {
        name: "Default Workspace".to_string(),
        description: Some("Default workspace for development".to_string()),
        keyboard_shortcut: "cmd+1".to_string(),  // Use text format expected by workspace validation
        tiling_pattern_id: Some(default_pattern_id),
        auto_arrange: Some(true),
    };

    match workspace_manager.create_workspace(create_request, default_pattern_id).await {
        Ok(workspace_id) => {
            info!("Created default workspace with ID: {}", workspace_id);
            
            // Get workspace count
            let count = workspace_manager.get_workspace_count().await;
            info!("Total workspaces: {}", count);
            
            // Get active workspace
            if let Some(active_workspace) = workspace_manager.get_active_workspace().await {
                info!("Active workspace: {} ({})", active_workspace.name, active_workspace.id);
            }
        }
        Err(e) => {
            error!("Failed to create default workspace: {}", e);
        }
    }

    // Demonstrate workspace search functionality
    let found_workspaces = workspace_manager.find_workspaces_by_name("default").await;
    info!("Found {} workspaces matching 'default'", found_workspaces.len());

    // In a real application, this is where we would:
    // 1. Initialize the macOS integration layer
    // 2. Start the keyboard handler for global shortcuts
    // 3. Begin monitoring window changes
    // 4. Enter the main event loop
    
    info!("TilleRS initialization complete");
    info!("Note: This is a demonstration build - full window management features require additional implementation");

    Ok(())
}

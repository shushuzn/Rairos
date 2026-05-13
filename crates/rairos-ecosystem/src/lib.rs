//! rairos-ecosystem — Digital ecosystem component registry.

#![allow(clippy::vec_init_then_push)]
//!
//! Ported from `core/ecosystem.py`.
//!
//! Inspired by Volkswagen's V2G ecosystem model.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;

/// Status of an ecosystem component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentStatus {
    Ready,
    Planned,
    ComingSoon,
}

/// Represents an ecosystem component.
#[derive(Debug, Clone)]
pub struct EcosystemComponent {
    pub name: String,
    pub description: String,
    pub icon: String,
    pub status: ComponentStatus,
    pub url: Option<String>,
}

/// Digital ecosystem for AI Research OS.
#[derive(Debug)]
pub struct Ecosystem {
    components: Mutex<HashMap<String, EcosystemComponent>>,
}

impl Default for Ecosystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Ecosystem {
    pub fn new() -> Self {
        let mut components = HashMap::new();
        components.insert(
            "cli".to_string(),
            EcosystemComponent {
                name: "命令行工具".to_string(),
                description: "完整的CLI工具集".to_string(),
                icon: "🖥️".to_string(),
                status: ComponentStatus::Ready,
                url: Some("python cli.py --help".to_string()),
            },
        );
        components.insert(
            "simple_cli".to_string(),
            EcosystemComponent {
                name: "简化CLI".to_string(),
                description: "新手友好的命令行界面".to_string(),
                icon: "🚀".to_string(),
                status: ComponentStatus::Ready,
                url: Some("python -m core.simple_cli help".to_string()),
            },
        );
        components.insert(
            "api".to_string(),
            EcosystemComponent {
                name: "Python API".to_string(),
                description: "完整的Python API".to_string(),
                icon: "📦".to_string(),
                status: ComponentStatus::Ready,
                url: Some("python -c 'from ai_research_os import ...'".to_string()),
            },
        );
        components.insert(
            "achievements".to_string(),
            EcosystemComponent {
                name: "成就系统".to_string(),
                description: "积分和徽章激励".to_string(),
                icon: "🏆".to_string(),
                status: ComponentStatus::Ready,
                url: Some("python -m core.achievements".to_string()),
            },
        );
        components.insert(
            "performance".to_string(),
            EcosystemComponent {
                name: "性能监控".to_string(),
                description: "实时性能保证".to_string(),
                icon: "🛡️".to_string(),
                status: ComponentStatus::Ready,
                url: Some("python -m core.performance_guarantee".to_string()),
            },
        );
        components.insert(
            "value".to_string(),
            EcosystemComponent {
                name: "价值量化".to_string(),
                description: "VW式价值计算".to_string(),
                icon: "💰".to_string(),
                status: ComponentStatus::Ready,
                url: Some("python -m core.value_quantifier".to_string()),
            },
        );
        components.insert(
            "setup_wizard".to_string(),
            EcosystemComponent {
                name: "快速设置".to_string(),
                description: "5分钟完成设置（VW需8-10周）".to_string(),
                icon: "⚡".to_string(),
                status: ComponentStatus::Ready,
                url: Some("python -m core.setup_wizard".to_string()),
            },
        );
        components.insert(
            "gui".to_string(),
            EcosystemComponent {
                name: "图形界面".to_string(),
                description: "Web界面规划中".to_string(),
                icon: "🌐".to_string(),
                status: ComponentStatus::ComingSoon,
                url: None,
            },
        );
        components.insert(
            "plugins".to_string(),
            EcosystemComponent {
                name: "插件系统".to_string(),
                description: "可扩展插件架构".to_string(),
                icon: "🔌".to_string(),
                status: ComponentStatus::Planned,
                url: None,
            },
        );
        components.insert(
            "marketplace".to_string(),
            EcosystemComponent {
                name: "插件市场".to_string(),
                description: "插件生态系统".to_string(),
                icon: "🛒".to_string(),
                status: ComponentStatus::Planned,
                url: None,
            },
        );
        Self {
            components: Mutex::new(components),
        }
    }

    /// Get a component by ID.
    pub fn get(&self, id: &str) -> Option<EcosystemComponent> {
        self.components.lock().unwrap().get(id).cloned()
    }

    /// Get all components.
    pub fn all_components(&self) -> Vec<(String, EcosystemComponent)> {
        self.components
            .lock()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Get components by status.
    pub fn by_status(&self, status: ComponentStatus) -> Vec<EcosystemComponent> {
        self.components
            .lock()
            .unwrap()
            .values()
            .filter(|c| c.status == status)
            .cloned()
            .collect()
    }

    /// Get ecosystem report as formatted string.
    pub fn get_report(&self) -> String {
        let ready = self.by_status(ComponentStatus::Ready);
        let planned = self.by_status(ComponentStatus::Planned);
        let coming = self.by_status(ComponentStatus::ComingSoon);

        let mut lines = Vec::new();
        lines.push("=".repeat(60));
        lines.push("🌐 数字生态系统报告 (Volkswagen式生态)".to_string());
        lines.push("=".repeat(60));
        lines.push(String::new());
        lines.push("Volkswagen V2G生态:".to_string());
        lines.push("  电动车 + App + 智能电表 + 能源市场".to_string());
        lines.push(String::new());
        lines.push("我们AI Research OS生态:".to_string());
        lines.push("-".repeat(60));

        lines.push(String::new());
        lines.push("✅ 已就绪:".to_string());
        for c in &ready {
            lines.push(format!("  {} {}", c.icon, c.name));
            lines.push(format!("     {}", c.description));
            if let Some(ref url) = c.url {
                lines.push(format!("     访问: {}", url));
            }
        }

        if !planned.is_empty() {
            lines.push(String::new());
            lines.push("🚧 规划中:".to_string());
            for c in &planned {
                lines.push(format!("  {} {}", c.icon, c.name));
                lines.push(format!("     {}", c.description));
            }
        }

        if !coming.is_empty() {
            lines.push(String::new());
            lines.push("🔮 即将推出:".to_string());
            for c in &coming {
                lines.push(format!("  {} {}", c.icon, c.name));
                lines.push(format!("     {}", c.description));
            }
        }

        lines.push(String::new());
        lines.push("=".repeat(60));
        lines.push(String::new());
        lines.push("💡 Volkswagen承诺完整生态，我们提供完整工具链！".to_string());
        lines.push("=".repeat(60));

        lines.join("\n")
    }
}

// ─── Global ecosystem ─────────────────────────────────────────────────────────

static GLOBAL_ECOSYSTEM: LazyLock<Ecosystem> = LazyLock::new(Ecosystem::new);

/// Get the global ecosystem.
pub fn get_ecosystem() -> &'static Ecosystem {
    &GLOBAL_ECOSYSTEM
}

/// Print ecosystem report to stdout.
pub fn print_report() {
    println!("{}", get_ecosystem().get_report());
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_ecosystem_has_components() {
        let eco = Ecosystem::new();
        let all = eco.all_components();
        assert_eq!(all.len(), 10);
    }

    #[test]
    fn test_by_status() {
        let eco = Ecosystem::new();
        let ready = eco.by_status(ComponentStatus::Ready);
        assert_eq!(ready.len(), 7);
        let planned = eco.by_status(ComponentStatus::Planned);
        assert_eq!(planned.len(), 2);
    }

    #[test]
    fn test_get_component() {
        let eco = Ecosystem::new();
        let cli = eco.get("cli").unwrap();
        assert_eq!(cli.name, "命令行工具");
        assert_eq!(cli.status, ComponentStatus::Ready);
    }

    #[test]
    fn test_report_contains_ready() {
        let eco = Ecosystem::new();
        let report = eco.get_report();
        assert!(report.contains("已就绪"));
        assert!(report.contains("命令行工具"));
    }

    #[test]
    fn test_report_contains_vw_reference() {
        let eco = Ecosystem::new();
        let report = eco.get_report();
        assert!(report.contains("Volkswagen"));
        assert!(report.contains("V2G"));
    }
}

//! rairos-achievements — User achievement and gamification system.
//!
//! Ported from `core/achievements.py`.

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An achievement or badge definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub points: i32,
    #[serde(default)]
    pub unlocked_at: Option<DateTime<Local>>,
}

/// User statistics tracked for achievement unlocking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserStats {
    pub papers_processed: i32,
    pub api_calls_saved: i32,
    pub hours_saved: f64,
    pub searches_performed: i32,
    pub imports_performed: i32,
}

/// Achievement system with points and badges.
#[derive(Debug, Clone)]
pub struct AchievementSystem {
    achievements: HashMap<String, Achievement>,
    total_points: i32,
    user_stats: UserStats,
}

impl Default for AchievementSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl AchievementSystem {
    /// Create a new achievement system with all achievement definitions.
    pub fn new() -> Self {
        let mut achievements = HashMap::new();

        achievements.insert(
            "first_import".to_string(),
            Achievement {
                id: "first_import".to_string(),
                name: "🚀 首次导入".to_string(),
                description: "成功导入第一篇论文".to_string(),
                icon: "📥".to_string(),
                points: 10,
                unlocked_at: None,
            },
        );
        achievements.insert(
            "paper_collector".to_string(),
            Achievement {
                id: "paper_collector".to_string(),
                name: "📚 论文收集者".to_string(),
                description: "导入10篇论文".to_string(),
                icon: "📚".to_string(),
                points: 50,
                unlocked_at: None,
            },
        );
        achievements.insert(
            "researcher_100".to_string(),
            Achievement {
                id: "researcher_100".to_string(),
                name: "🎓 研究达人".to_string(),
                description: "导入100篇论文".to_string(),
                icon: "🎓".to_string(),
                points: 200,
                unlocked_at: None,
            },
        );
        achievements.insert(
            "api_saver".to_string(),
            Achievement {
                id: "api_saver".to_string(),
                name: "💰 API节流侠".to_string(),
                description: "通过缓存节省100次API调用".to_string(),
                icon: "💰".to_string(),
                points: 100,
                unlocked_at: None,
            },
        );
        achievements.insert(
            "time_saver".to_string(),
            Achievement {
                id: "time_saver".to_string(),
                name: "⏰ 时间管理大师".to_string(),
                description: "节省10小时研究时间".to_string(),
                icon: "⏰".to_string(),
                points: 150,
                unlocked_at: None,
            },
        );
        achievements.insert(
            "speed_demon".to_string(),
            Achievement {
                id: "speed_demon".to_string(),
                name: "⚡ 速度达人".to_string(),
                description: "批量导入50篇论文".to_string(),
                icon: "⚡".to_string(),
                points: 100,
                unlocked_at: None,
            },
        );
        achievements.insert(
            "cache_master".to_string(),
            Achievement {
                id: "cache_master".to_string(),
                name: "🗄️ 缓存大师".to_string(),
                description: "缓存命中率超过80%".to_string(),
                icon: "🗄️".to_string(),
                points: 75,
                unlocked_at: None,
            },
        );
        achievements.insert(
            "search_expert".to_string(),
            Achievement {
                id: "search_expert".to_string(),
                name: "🔍 搜索专家".to_string(),
                description: "执行100次搜索".to_string(),
                icon: "🔍".to_string(),
                points: 50,
                unlocked_at: None,
            },
        );

        Self {
            achievements,
            total_points: 0,
            user_stats: UserStats::default(),
        }
    }

    /// Unlock an achievement by ID. Returns the achievement if newly unlocked, None otherwise.
    pub fn unlock_achievement(&mut self, achievement_id: &str) -> Option<Achievement> {
        let achievement = self.achievements.get_mut(achievement_id)?;

        if achievement.unlocked_at.is_none() {
            achievement.unlocked_at = Some(Local::now());
            self.total_points += achievement.points;
            return Some(achievement.clone());
        }
        None
    }

    /// Check and auto-unlock achievements based on current stats.
    /// Returns a list of newly unlocked achievements.
    pub fn check_achievements(&mut self) -> Vec<Achievement> {
        let mut unlocked = Vec::new();

        // Check first import
        if self.user_stats.imports_performed >= 1 {
            if let Some(a) = self.unlock_achievement("first_import") {
                unlocked.push(a);
            }
        }

        // Check paper collector (10 papers)
        if self.user_stats.papers_processed >= 10 {
            if let Some(a) = self.unlock_achievement("paper_collector") {
                unlocked.push(a);
            }
        }

        // Check researcher (100 papers)
        if self.user_stats.papers_processed >= 100 {
            if let Some(a) = self.unlock_achievement("researcher_100") {
                unlocked.push(a);
            }
        }

        // Check API saver
        if self.user_stats.api_calls_saved >= 100 {
            if let Some(a) = self.unlock_achievement("api_saver") {
                unlocked.push(a);
            }
        }

        // Check time saver (10 hours)
        if self.user_stats.hours_saved >= 10.0 {
            if let Some(a) = self.unlock_achievement("time_saver") {
                unlocked.push(a);
            }
        }

        // Check speed demon (50 papers)
        if self.user_stats.papers_processed >= 50 {
            if let Some(a) = self.unlock_achievement("speed_demon") {
                unlocked.push(a);
            }
        }

        unlocked
    }

    /// Update user statistics. Auto-checks achievements after update.
    /// Returns newly unlocked achievements.
    pub fn update_stats(
        &mut self,
        papers_processed: Option<i32>,
        api_calls_saved: Option<i32>,
        hours_saved: Option<f64>,
        searches_performed: Option<i32>,
        imports_performed: Option<i32>,
    ) -> Vec<Achievement> {
        if let Some(v) = papers_processed {
            self.user_stats.papers_processed = v;
        }
        if let Some(v) = api_calls_saved {
            self.user_stats.api_calls_saved = v;
        }
        if let Some(v) = hours_saved {
            self.user_stats.hours_saved = v;
        }
        if let Some(v) = searches_performed {
            self.user_stats.searches_performed = v;
        }
        if let Some(v) = imports_performed {
            self.user_stats.imports_performed = v;
        }

        self.check_achievements()
    }

    /// Get all unlocked achievements.
    pub fn get_unlocked_achievements(&self) -> Vec<&Achievement> {
        self.achievements
            .values()
            .filter(|a| a.unlocked_at.is_some())
            .collect()
    }

    /// Get all pending (not yet unlocked) achievements.
    pub fn get_pending_achievements(&self) -> Vec<&Achievement> {
        self.achievements
            .values()
            .filter(|a| a.unlocked_at.is_none())
            .collect()
    }

    /// Get progress report as formatted string.
    pub fn get_progress_report(&self) -> String {
        let unlocked = self.get_unlocked_achievements();
        let pending = self.get_pending_achievements();
        let total = self.achievements.len();

        let mut lines = Vec::new();
        lines.push("=".repeat(60));
        lines.push("🏆 成就报告".to_string());
        lines.push("=".repeat(60));
        lines.push(String::new());
        lines.push(format!("总积分: {}", self.total_points));
        lines.push(format!("已解锁成就: {}/{}", unlocked.len(), total));
        lines.push(String::new());
        lines.push("📊 使用统计:".to_string());
        lines.push(format!(
            "  处理论文数: {}",
            self.user_stats.papers_processed
        ));
        lines.push(format!(
            "  节省API调用: {}",
            self.user_stats.api_calls_saved
        ));
        lines.push(format!(
            "  节省时间: {:.1} 小时",
            self.user_stats.hours_saved
        ));
        lines.push(String::new());
        lines.push("🏅 已解锁成就:".to_string());

        if unlocked.is_empty() {
            lines.push("  暂无解锁成就".to_string());
        } else {
            for a in &unlocked {
                lines.push(format!("  {} {} (+{}分)", a.icon, a.name, a.points));
            }
        }

        lines.push(String::new());
        lines.push("🎯 即将解锁:".to_string());
        for a in pending.iter().take(3) {
            lines.push(format!("  {} {} ({})", a.icon, a.name, a.description));
        }

        lines.push(String::new());
        lines.push("=".repeat(60));

        lines.join("\n")
    }

    /// Get value saved as a map.
    pub fn get_value_saved(&self) -> HashMap<String, String> {
        let hours_saved = self.user_stats.api_calls_saved as f64 * 0.1;
        let cost_saved = self.user_stats.api_calls_saved as f64 * 0.01;

        let mut map = HashMap::new();
        map.insert(
            "hours_saved".to_string(),
            format!("{:.1} 小时", hours_saved),
        );
        map.insert("cost_saved".to_string(), format!("${:.2}", cost_saved));
        map.insert(
            "papers_processed".to_string(),
            self.user_stats.papers_processed.to_string(),
        );
        map.insert(
            "achievement_points".to_string(),
            self.total_points.to_string(),
        );
        map
    }

    /// Get total achievement points.
    pub fn total_points(&self) -> i32 {
        self.total_points
    }

    /// Get a reference to current user stats.
    pub fn user_stats(&self) -> &UserStats {
        &self.user_stats
    }

    /// Get an achievement by ID (cloned).
    pub fn get_achievement(&self, id: &str) -> Option<Achievement> {
        self.achievements.get(id).cloned()
    }

    /// Get all achievement definitions.
    pub fn all_achievements(&self) -> Vec<&Achievement> {
        self.achievements.values().collect()
    }
}

// ─── Global system ──────────────────────────────────────────────────────────────

use std::sync::LazyLock;
use std::sync::Mutex;

static GLOBAL_SYSTEM: LazyLock<Mutex<AchievementSystem>> =
    LazyLock::new(|| Mutex::new(AchievementSystem::new()));

/// Get the global achievement system.
pub fn get_achievement_system() -> std::sync::MutexGuard<'static, AchievementSystem> {
    GLOBAL_SYSTEM.lock().unwrap()
}

/// Print the achievement report to stdout.
pub fn print_achievement_report() {
    let system = get_achievement_system();
    println!("{}", system.get_progress_report());
    println!();
    println!("💰 价值量化:");
    for (key, val) in system.get_value_saved() {
        println!("  {}: {}", key, val);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_system_has_no_unlocked() {
        let system = AchievementSystem::new();
        assert!(system.get_unlocked_achievements().is_empty());
        assert_eq!(system.total_points(), 0);
        assert_eq!(system.all_achievements().len(), 8);
    }

    #[test]
    fn test_unlock_first_import() {
        let mut system = AchievementSystem::new();
        let unlocked = system.unlock_achievement("first_import");
        assert!(unlocked.is_some());
        assert_eq!(unlocked.unwrap().points, 10);
        assert_eq!(system.total_points(), 10);

        // Can't unlock twice
        let second = system.unlock_achievement("first_import");
        assert!(second.is_none());
        assert_eq!(system.total_points(), 10);
    }

    #[test]
    fn test_check_achievements_auto_unlock() {
        let mut system = AchievementSystem::new();
        system.user_stats.papers_processed = 10;
        system.user_stats.imports_performed = 1;
        let unlocked = system.check_achievements();

        // Should unlock first_import (imports >= 1) and paper_collector (10 papers)
        assert!(unlocked.iter().any(|a| a.id == "first_import"));
        assert!(unlocked.iter().any(|a| a.id == "paper_collector"));
        assert_eq!(system.total_points(), 10 + 50);
    }

    #[test]
    fn test_update_stats_auto_check() {
        let mut system = AchievementSystem::new();
        let unlocked = system.update_stats(Some(100), Some(200), Some(15.0), None, Some(5));

        // Should unlock: first_import, paper_collector, researcher_100, api_saver, time_saver
        assert!(unlocked.iter().any(|a| a.id == "first_import"));
        assert!(unlocked.iter().any(|a| a.id == "paper_collector"));
        assert!(unlocked.iter().any(|a| a.id == "researcher_100"));
        assert!(unlocked.iter().any(|a| a.id == "api_saver"));
        assert!(unlocked.iter().any(|a| a.id == "time_saver"));
        // speed_demon requires 50 papers (met)
        assert!(unlocked.iter().any(|a| a.id == "speed_demon"));

        assert_eq!(system.user_stats.papers_processed, 100);
        assert_eq!(system.user_stats.api_calls_saved, 200);
    }

    #[test]
    fn test_get_pending_achievements() {
        let system = AchievementSystem::new();
        let pending = system.get_pending_achievements();
        assert_eq!(pending.len(), 8);
    }

    #[test]
    fn test_get_value_saved() {
        let mut system = AchievementSystem::new();
        system.user_stats.api_calls_saved = 100;
        system.user_stats.papers_processed = 50;

        let value = system.get_value_saved();
        assert_eq!(value.get("hours_saved").unwrap(), "10.0 小时");
        assert_eq!(value.get("cost_saved").unwrap(), "$1.00");
        assert_eq!(value.get("papers_processed").unwrap(), "50");
    }

    #[test]
    fn test_progress_report_contains_stats() {
        let system = AchievementSystem::new();
        let report = system.get_progress_report();
        assert!(report.contains("总积分: 0"));
        assert!(report.contains("已解锁成就: 0/8"));
        assert!(report.contains("处理论文数: 0"));
    }
}

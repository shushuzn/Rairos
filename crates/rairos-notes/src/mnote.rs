//! M-Note (comparison note) management.

use crate::render::render_mnote;
use glob::glob;
use regex::Regex;
use std::path::{Path, PathBuf};

fn short(stem: &str, n: usize) -> String {
    let re = Regex::new(r"^P\s*-\s*\d{4}\s*-\s*").unwrap();
    let s = re.replace(stem, "").trim().to_string();
    if s.len() <= n {
        return s;
    }
    let truncated = s[..n - 5]
        .trim_end_matches('-')
        .trim_end_matches('_')
        .trim_end_matches(' ')
        .to_string();
    let suffix = format!("{:05}", truncated.hash64() % 100000);
    format!("{}~{}", truncated, suffix)
}

trait Hash64 {
    fn hash64(&self) -> u64;
}

impl Hash64 for str {
    fn hash64(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

pub fn mnote_filename(tag: &str, a: &Path, b: &Path, c: &Path) -> String {
    let A = short(a.file_stem().unwrap().to_string_lossy().as_ref(), 19);
    let B = short(b.file_stem().unwrap().to_string_lossy().as_ref(), 19);
    let C = short(c.file_stem().unwrap().to_string_lossy().as_ref(), 19);
    format!("M - {tag} - {A} vs {B} vs {C}.md")
}

fn parse_current_abc(md: &str) -> (Option<String>, Option<String>, Option<String>) {
    fn find(label: &str, md: &str) -> Option<String> {
        let pattern = format!(r"^\-\s*{}:\s*(.+)\s*$", regex::escape(label));
        let re = Regex::new(&pattern).unwrap();
        re.captures(md)
            .map(|c| c.get(1).unwrap().as_str().trim().to_string())
    }
    (find("A", md), find("B", md), find("C", md))
}

fn today_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

fn append_view_evolution_log(
    md: &str,
    old_abc: (Option<String>, Option<String>, Option<String>),
    new_abc: (Option<String>, Option<String>, Option<String>),
) -> String {
    let today = today_iso();
    let block = format!(
        r#"

* {today}

  * 旧观点：A/B/C = {}/{}/{}
  * 新证据：新增/更新同主题论文，A/B/C 刷新为 {}/{}/{}
  * 更新结论：

"#,
        old_abc.0.as_deref().unwrap_or("?"),
        old_abc.1.as_deref().unwrap_or("?"),
        old_abc.2.as_deref().unwrap_or("?"),
        new_abc.0.as_deref().unwrap_or("?"),
        new_abc.1.as_deref().unwrap_or("?"),
        new_abc.2.as_deref().unwrap_or("?"),
    );

    let re = Regex::new(r"^##\s+View Evolution Log\s*$").unwrap();
    if let Some(m) = re.find(md) {
        let insert_pos = m.end();
        format!(
            "{}{}{}{}",
            &md[..insert_pos],
            "\n",
            block,
            &md[insert_pos..]
        )
    } else {
        format!("{}\n\n## View Evolution Log\n{}", md.trim_end(), block)
    }
}

pub fn ensure_or_update_mnote(
    mnote_dir: &Path,
    tag: &str,
    top3: &[PathBuf],
) -> Option<PathBuf> {
    std::fs::create_dir_all(mnote_dir).ok()?;
    if top3.len() < 3 {
        return None;
    }

    let pattern = mnote_dir.join(format!("M - {tag} - *.md"));
    let existing: Vec<_> = glob(&pattern.to_string_lossy())
        .ok()?
        .filter_map(|p| p.ok())
        .filter(|p| p.is_file())
        .collect();
    let mut existing = existing;
    existing.sort();

    let a = &top3[0];
    let b = &top3[1];
    let c = &top3[2];
    let newA = a.file_stem().unwrap().to_string_lossy();
    let newB = b.file_stem().unwrap().to_string_lossy();
    let newC = c.file_stem().unwrap().to_string_lossy();

    if existing.is_empty() {
        let fname = mnote_filename(tag, a, b, c);
        let path = mnote_dir.join(&fname);
        let title = format!("{tag}: {newA} vs {newB} vs {newC}");
        let _ = std::fs::write(&path, render_mnote(&title, &newA, &newB, &newC));
        return Some(path);
    }

    let path = &existing[0];
    let md = std::fs::read_to_string(path).unwrap_or_default();
    let (curA, curB, curC) = parse_current_abc(&md);

    if curA.is_none() || curB.is_none() || curC.is_none() {
        let md2 = format!(
            "{}\n\n---\n\n## 当前 A/B/C（自动补齐）\n\n- A: {}\n- B: {}\n- C: {}\n",
            md.trim_end(),
            newA,
            newB,
            newC
        );
        let _ = std::fs::write(path, md2);
        return Some(path.clone());
    }

    if (curA.as_deref(), curB.as_deref(), curC.as_deref())
        != (
            Some(newA.as_ref()),
            Some(newB.as_ref()),
            Some(newC.as_ref()),
        )
    {
        let re_a = Regex::new(&format!(r"^\-\s*A:\s*.*$")).unwrap();
        let re_b = Regex::new(&format!(r"^\-\s*B:\s*.*$")).unwrap();
        let re_c = Regex::new(&format!(r"^\-\s*C:\s*.*$")).unwrap();
        let mut md2 = re_a.replace(&md, format!("- A: {newA}")).to_string();
        md2 = re_b.replace(&md2, format!("- B: {newB}")).to_string();
        md2 = re_c.replace(&md2, format!("- C: {newC}")).to_string();
        md2 = append_view_evolution_log(
            &md2,
            (curA, curB, curC),
            (
                Some(newA.to_string()),
                Some(newB.to_string()),
                Some(newC.to_string()),
            ),
        );
        let _ = std::fs::write(path, md2);
    }

    Some(path.clone())
}

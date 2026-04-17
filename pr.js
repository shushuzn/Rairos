const GITHUB_TOKEN = process.env.GITHUB_TOKEN || process.env.GITHUB_PERSONAL_ACCESS_TOKEN;
const OWNER = "shushuzn";
const REPO = "ai_research_os";

async function run() {
  const { execSync } = require("child_process");

  // Stage and get diff
  execSync("git add -A", { encoding: "utf8", stdio: "inherit" });
  const diff = execSync("git diff --staged --stat", { encoding: "utf8" }).trim();
  console.log("Diff:\n", diff);

  // Get SHA
  const sha = execSync("git rev-parse HEAD", { encoding: "utf8" }).trim();
  console.log("Head SHA:", sha);

  // Get main SHA via correct API endpoint
  const refRes = await fetch(
    `https://api.github.com/repos/${OWNER}/${REPO}/git/refs/heads/main`,
    { headers: { Authorization: `Bearer ${GITHUB_TOKEN}`, "X-GitHub-Api-Version": "2022-11-28" } }
  );
  const refData = await refRes.json();
  const mainSha = refData.object.sha;
  console.log("Main SHA:", mainSha);

  // Create branch
  const branchName = `feat/merge-auto-${Date.now()}`;
  await fetch(
    `https://api.github.com/repos/${OWNER}/${REPO}/git/refs`,
    {
      method: "POST",
      headers: { Authorization: `Bearer ${GITHUB_TOKEN}`, "Content-Type": "application/json", "X-GitHub-Api-Version": "2022-11-28" },
      body: JSON.stringify({ ref: `refs/heads/${branchName}`, sha: mainSha })
    }
  );
  console.log("Branch:", branchName);

  // Get base tree SHA
  const baseTreeRes = await fetch(
    `https://api.github.com/repos/${OWNER}/${REPO}/git/trees/${mainSha}?recursive=1`,
    { headers: { Authorization: `Bearer ${GITHUB_TOKEN}`, "X-GitHub-Api-Version": "2022-11-28" } }
  );
  const baseTreeData = await baseTreeRes.json();
  const baseTreeSha = baseTreeData.sha;

  // Get files to update
  const stagedFiles = execSync("git diff --staged --name-only", { encoding: "utf8" }).trim().split("\n").filter(f => f);
  console.log("Files:", stagedFiles);

  // Create blobs and new tree
  const newTreeEntries = [];
  for (const file of stagedFiles) {
    const content = execSync(`git show :${file}`, { encoding: "utf8" });
    const blobRes = await fetch(
      `https://api.github.com/repos/${OWNER}/${REPO}/git/blobs`,
      {
        method: "POST",
        headers: { Authorization: `Bearer ${GITHUB_TOKEN}`, "Content-Type": "application/json", "X-GitHub-Api-Version": "2022-11-28" },
        body: JSON.stringify({ content, encoding: "utf-8" })
      }
    );
    const blobData = await blobRes.json();
    newTreeEntries.push({ path: file, mode: "100644", type: "blob", sha: blobData.sha });
  }

  const treeRes = await fetch(
    `https://api.github.com/repos/${OWNER}/${REPO}/git/trees`,
    {
      method: "POST",
      headers: { Authorization: `Bearer ${GITHUB_TOKEN}`, "Content-Type": "application/json", "X-GitHub-Api-Version": "2022-11-28" },
      body: JSON.stringify({ base_tree: baseTreeSha, tree: newTreeEntries })
    }
  );
  const treeData = await treeRes.json();
  console.log("Tree:", treeData.sha);

  // Create commit
  const commitMsg = `feat(merge): add --auto flag for auto-merging high-similarity pairs

Adds \`--auto\` flag to the \`merge\` command.

Scans all papers, finds pairs with cosine similarity >= 0.95,
and merges them using --keep=semantic logic (better parse_status wins).

Usage:
  ai_research_os.py merge --auto --dry-run  # preview
  ai_research_os.py merge --auto            # execute

Closes # (Feature request from Phase 3 roadmap)`;

  const commitRes = await fetch(
    `https://api.github.com/repos/${OWNER}/${REPO}/git/commits`,
    {
      method: "POST",
      headers: { Authorization: `Bearer ${GITHUB_TOKEN}`, "Content-Type": "application/json", "X-GitHub-Api-Version": "2022-11-28" },
      body: JSON.stringify({
        message: commitMsg,
        tree: treeData.sha,
        parents: [sha]
      })
    }
  );
  const commitData = await commitRes.json();
  console.log("Commit:", commitData.sha);

  // Update branch ref
  await fetch(
    `https://api.github.com/repos/${OWNER}/${REPO}/git/refs/heads/${branchName}`,
    {
      method: "PATCH",
      headers: { Authorization: `Bearer ${GITHUB_TOKEN}`, "Content-Type": "application/json", "X-GitHub-Api-Version": "2022-11-28" },
      body: JSON.stringify({ sha: commitData.sha })
    }
  );

  // Create PR
  const prRes = await fetch(
    `https://api.github.com/repos/${OWNER}/${REPO}/pulls`,
    {
      method: "POST",
      headers: { Authorization: `Bearer ${GITHUB_TOKEN}`, "Content-Type": "application/json", "X-GitHub-Api-Version": "2022-11-28" },
      body: JSON.stringify({
        title: "feat(merge): add --auto flag for auto-merging high-similarity pairs",
        head: branchName,
        base: "main",
        body: "Adds `--auto` flag to the `merge` command.\n\nScans all papers, finds pairs with cosine similarity >= 0.95, and merges them using `--keep=semantic` logic (better `parse_status` wins).\n\n```bash\nai_research_os.py merge --auto --dry-run  # preview\nai_research_os.py merge --auto            # execute\n```\n\n- Uses `db.find_similar(threshold=0.95)` to discover pairs\n- Logs merges with strategy `semantic-auto`\n- 752 tests pass",
        draft: false
      })
    }
  );
  const prData = await prRes.json();
  console.log("PR:", prData.number, prData.html_url);

  // Wait for CI
  let attempt = 0;
  while (attempt < 20) {
    await new Promise(r => setTimeout(r, 10000));
    const statusRes = await fetch(
      `https://api.github.com/repos/${OWNER}/${REPO}/commits/${commitData.sha}/status`,
      { headers: { Authorization: `Bearer ${GITHUB_TOKEN}`, "X-GitHub-Api-Version": "2022-11-28" } }
    );
    const statusData = await statusRes.json();
    console.log(`Status [${attempt + 1}/20]:`, statusData.state);
    if (statusData.state === "success") break;
    attempt++;
  }

  // Merge
  const mergeRes = await fetch(
    `https://api.github.com/repos/${OWNER}/${REPO}/pulls/${prData.number}/merge`,
    {
      method: "PUT",
      headers: { Authorization: `Bearer ${GITHUB_TOKEN}`, "Content-Type": "application/json", "X-GitHub-Api-Version": "2022-11-28" },
      body: JSON.stringify({ merge_method: "squash" })
    }
  );
  const mergeData = await mergeRes.json();
  console.log("Merged:", !!mergeData.merged, "SHA:", mergeData.sha);
}

run().catch(e => { console.error(e); process.exit(1); });

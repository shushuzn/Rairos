"""Fix gene_pool_backup route."""
with open("web/routes_gene_pool.py", encoding="utf-8") as f:
    c = f.read()

old = '''def gene_pool_backup(request: Request):
    """Gene Pool Backup - create and restore snapshots."""
    html = "<p>No backups yet</p>"
    try:
        from llm.gene_pool_backup import get_backup_info
        info = get_backup_info()
        stamps = info.get("stamps", []) if isinstance(info, dict) else info
        html = _render_backup_html(stamps if isinstance(stamps, list) else [])
    except Exception:
        pass'''

new = '''def gene_pool_backup(request: Request):
    """Gene Pool Backup - create and restore snapshots."""
    html = "<p>No backups yet</p>"
    try:
        from llm.gene_pool_backup import get_backup_info, create_backup
        info = get_backup_info()
        count = info.get("available", 0) if isinstance(info, dict) else 0
        stamps = info.get("stamps", []) if isinstance(info, dict) else []
        if stamps:
            rows = ''.join(f'<tr><td>{s}</td><td><form action=\"/gene-pool/backup/restore/{s.replace(\".tar\",\"\")}\" method=\"post\" style=\"display:inline\"><button class=\"btn\" style=\"font-size:12px;padding:2px 10px;\">Restore</button></form></td></tr>' for s in stamps)
            html = f'<table class=\"credibility-table\"><thead><tr><th>Backup</th><th></th></tr></thead><tbody>{rows}</tbody></table>'
        html += '<form action=\"/gene-pool/backup/create\" method=\"post\" style=\"margin-top:16px\"><button class=\"btn btn-primary\">Create Backup</button></form>'
    except Exception:
        pass'''

c = c.replace(old, new)

with open("web/routes_gene_pool.py", "w", encoding="utf-8") as f:
    f.write(c)

import py_compile
py_compile.compile("web/routes_gene_pool.py", doraise=True)
print("OK")

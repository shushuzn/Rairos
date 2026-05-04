"""Fix gene_pool_backup route to handle dict response."""
with open("web/routes_gene_pool.py", encoding="utf-8") as f:
    c = f.read()

old = '''def gene_pool_backup(request: Request):
    """Gene Pool Backup - create and restore snapshots."""
    html = "<p>Backup module unavailable</p>"
    try:
        from llm.gene_pool_backup import get_backup_info
        backups = get_backup_info()
        if isinstance(backups, list):
            html = _render_backup_html(backups)
    except Exception:
        pass'''

new = '''def gene_pool_backup(request: Request):
    """Gene Pool Backup - create and restore snapshots."""
    html = "<p>No backups yet</p>"
    try:
        from llm.gene_pool_backup import get_backup_info
        info = get_backup_info()
        stamps = info.get("stamps", []) if isinstance(info, dict) else info
        html = _render_backup_html(stamps if isinstance(stamps, list) else [])
    except Exception:
        pass'''

c = c.replace(old, new)

with open("web/routes_gene_pool.py", "w", encoding="utf-8") as f:
    f.write(c)

import py_compile
py_compile.compile("web/routes_gene_pool.py", doraise=True)
print("OK")

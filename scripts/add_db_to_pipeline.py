"""Patch: add _get_db method to PaperPipeline"""
with open('research_loop/paper2code_integration/__init__.py', 'r', encoding='utf-8') as f:
    content = f.read()

old = '''    def _get_tracker(self, skip_gene_pool: bool):
        """Get EvolutionTracker for Gene Pool encoding."""
        if skip_gene_pool:
            return None

        try:
            from llm.insight.tracker import EvolutionTracker

            data_dir = self.tracker_data_dir or Path.home() / ".ai_research_os" / "evolution"
            return EvolutionTracker(data_dir=data_dir)
        except Exception as e:
            print(f"[paper2code] Warning: could not init EvolutionTracker: {e}")
            return None'''

new = '''    def _get_tracker(self, skip_gene_pool: bool):
        """Get EvolutionTracker for Gene Pool encoding."""
        if skip_gene_pool:
            return None

        try:
            from llm.insight.tracker import EvolutionTracker

            data_dir = self.tracker_data_dir or Path.home() / ".ai_research_os" / "evolution"
            return EvolutionTracker(data_dir=data_dir)
        except Exception as e:
            print(f"[paper2code] Warning: could not init EvolutionTracker: {e}")
            return None

    def _get_db(self):
        """Get Database instance for lineage tracking."""
        try:
            from db.database import Database
            db = Database()
            db.init()
            return db
        except Exception as e:
            print(f"[paper2code] Warning: could not init Database: {e}")
            return None'''

if old in content:
    content = content.replace(old, new, 1)
    print('Patched _get_db OK')
else:
    print('WARNING: pattern not found')

with open('research_loop/paper2code_integration/__init__.py', 'w', encoding='utf-8', newline='\n') as f:
    f.write(content)
print(f'File size: {len(content)}')

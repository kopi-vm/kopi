#!/usr/bin/env python3
"""
Display TDL traceability status by parsing Links sections in documents.
No frontmatter required - uses existing Links sections.
"""

import os
import re
import sys
from pathlib import Path
from collections import defaultdict
from typing import Dict, List, Set, Tuple

class TDLDocument:
    """Represents a TDL document with its relationships."""
    
    def __init__(self, path: Path):
        self.path = path
        self.filename = path.name
        self.doc_id = self.extract_id(self.filename)
        self.doc_type = self.extract_type(self.filename)
        self.links = self.parse_links(path)
        self.status = self.extract_status(path)
    
    @staticmethod
    def extract_id(filename: str) -> str:
        """Extract document ID from filename (e.g., FR-0001-... → FR-0001)."""
        match = re.match(r'^([A-Z]+-[^-]+)', filename)
        return match.group(1) if match else filename
    
    @staticmethod
    def extract_type(filename: str) -> str:
        """Extract document type from filename."""
        if filename.startswith('AN-'):
            return 'analysis'
        elif filename.startswith('FR-'):
            return 'requirement'
        elif filename.startswith('NFR-'):
            return 'requirement'
        elif filename.startswith('ADR-'):
            return 'adr'
        elif filename.startswith('T-'):
            return 'task'
        else:
            return 'unknown'
    
    @staticmethod
    def parse_links(path: Path) -> Dict[str, List[str]]:
        """Parse Links section from markdown file."""
        links = defaultdict(list)
        
        try:
            content = path.read_text(encoding='utf-8')
            
            # Find Links section
            links_match = re.search(r'## Links\s*\n(.*?)(?=\n##|\Z)', content, re.DOTALL)
            if not links_match:
                return links
            
            links_content = links_match.group(1)
            
            # Extract different link types
            # Pattern: - Category: [`ID`](path) or just ID references
            patterns = [
                (r'Formal Requirements?:\s*(.*?)(?=\n-|\n##|\Z)', 'requirements'),
                (r'Requirements?:\s*(.*?)(?=\n-|\n##|\Z)', 'requirements'),
                (r'Related ADRs?:\s*(.*?)(?=\n-|\n##|\Z)', 'adrs'),
                (r'Related Analyses?:\s*(.*?)(?=\n-|\n##|\Z)', 'analyses'),
                (r'Design:\s*(.*?)(?=\n-|\n##|\Z)', 'design'),
                (r'Plan:\s*(.*?)(?=\n-|\n##|\Z)', 'plan'),
            ]
            
            for pattern, link_type in patterns:
                match = re.search(pattern, links_content, re.DOTALL)
                if match:
                    # Extract IDs from the matched content
                    ids = re.findall(r'[A-Z]+-[0-9a-z]{4,5}(?:-[^/\s\]]+)?|[A-Z]+-\d+', match.group(1))
                    links[link_type].extend(ids)
            
        except Exception as e:
            print(f"Warning: Failed to parse {path}: {e}", file=sys.stderr)
        
        return dict(links)
    
    @staticmethod
    def extract_status(path: Path) -> str:
        """Extract status from document metadata."""
        try:
            content = path.read_text(encoding='utf-8')
            
            # Look for Status in metadata section
            status_match = re.search(r'^- Status:\s*(.+)$', content, re.MULTILINE)
            if status_match:
                return status_match.group(1).strip()
            
        except Exception:
            pass
        
        return 'Unknown'


class TraceabilityAnalyzer:
    """Analyze TDL document relationships and report status."""
    
    def __init__(self, repo_root: Path):
        self.repo_root = repo_root
        self.documents = {}
        self.load_documents()
    
    def load_documents(self):
        """Load all TDL documents."""
        doc_paths = [
            (self.repo_root / 'docs' / 'analysis', '*.md'),
            (self.repo_root / 'docs' / 'requirements', '*.md'),
            (self.repo_root / 'docs' / 'adr', '*.md'),
            (self.repo_root / 'docs' / 'tasks', '**/plan.md'),
            (self.repo_root / 'docs' / 'tasks', '**/design.md'),
        ]
        
        for base_path, pattern in doc_paths:
            if not base_path.exists():
                continue
            
            if '**' in pattern:
                files = base_path.glob(pattern)
            else:
                files = base_path.glob(pattern)
            
            for file_path in files:
                # Skip traceability.md and templates
                if 'traceability.md' in str(file_path) or 'templates/' in str(file_path):
                    continue
                
                doc = TDLDocument(file_path)
                self.documents[doc.doc_id] = doc
    
    def find_implementing_tasks(self, req_id: str) -> List[str]:
        """Find tasks that implement a requirement."""
        tasks = []
        for doc_id, doc in self.documents.items():
            if doc.doc_type == 'task' and req_id in doc.links.get('requirements', []):
                tasks.append(doc_id)
        return tasks
    
    def find_orphan_requirements(self) -> List[str]:
        """Find requirements without implementing tasks."""
        orphans = []
        for doc_id, doc in self.documents.items():
            if doc.doc_type == 'requirement':
                if not self.find_implementing_tasks(doc_id):
                    orphans.append(doc_id)
        return orphans
    
    def find_orphan_tasks(self) -> List[str]:
        """Find tasks without linked requirements."""
        orphans = []
        for doc_id, doc in self.documents.items():
            if doc.doc_type == 'task' and not doc.links.get('requirements'):
                orphans.append(doc_id)
        return orphans
    
    def calculate_coverage(self) -> Dict[str, any]:
        """Calculate coverage metrics."""
        requirements = [d for d in self.documents.values() if d.doc_type == 'requirement']
        tasks = [d for d in self.documents.values() if d.doc_type == 'task']
        analyses = [d for d in self.documents.values() if d.doc_type == 'analysis']
        adrs = [d for d in self.documents.values() if d.doc_type == 'adr']
        
        req_with_tasks = sum(1 for r in requirements if self.find_implementing_tasks(r.doc_id))
        
        return {
            'total_requirements': len(requirements),
            'total_tasks': len(tasks),
            'total_analyses': len(analyses),
            'total_adrs': len(adrs),
            'requirements_with_tasks': req_with_tasks,
            'coverage_percentage': (req_with_tasks / len(requirements) * 100) if requirements else 0,
        }
    
    def print_status(self, gaps_only: bool = False):
        """Print traceability status."""
        if not gaps_only:
            print("=== Kopi TDL Status ===\n")
            
            # Coverage metrics
            coverage = self.calculate_coverage()
            print("Coverage:")
            print(f"  Documents: {coverage['total_analyses']} analyses, "
                  f"{coverage['total_requirements']} requirements, "
                  f"{coverage['total_adrs']} ADRs, "
                  f"{coverage['total_tasks']} tasks")
            print(f"  Implementation: {coverage['requirements_with_tasks']}/{coverage['total_requirements']} "
                  f"requirements have tasks ({coverage['coverage_percentage']:.0f}%)")
            print()
        
        # Gaps
        orphan_reqs = self.find_orphan_requirements()
        orphan_tasks = self.find_orphan_tasks()
        
        if orphan_reqs or orphan_tasks:
            print("Gaps:")
            for req_id in sorted(orphan_reqs):
                doc = self.documents[req_id]
                print(f"  ⚠ {req_id}: No implementing task (Status: {doc.status})")
            for task_id in sorted(orphan_tasks):
                print(f"  ⚠ {task_id}: No linked requirements")
            print()
        elif not gaps_only:
            print("✓ No gaps detected\n")
        
        if not gaps_only:
            # Status summary by type
            print("Status by Document Type:")
            
            # Group by type
            by_type = defaultdict(list)
            for doc in self.documents.values():
                by_type[doc.doc_type].append(doc)
            
            for doc_type in ['analysis', 'requirement', 'adr', 'task']:
                if doc_type in by_type:
                    docs = by_type[doc_type]
                    print(f"\n  {doc_type.title()}s:")
                    
                    # Group by status
                    by_status = defaultdict(int)
                    for doc in docs:
                        by_status[doc.status] += 1
                    
                    for status, count in sorted(by_status.items()):
                        print(f"    {status}: {count}")
    
    def check_integrity(self) -> bool:
        """Check integrity for CI (return True if all good)."""
        orphan_reqs = self.find_orphan_requirements()
        orphan_tasks = self.find_orphan_tasks()
        
        if orphan_reqs or orphan_tasks:
            print("Traceability gaps detected:", file=sys.stderr)
            for req_id in orphan_reqs:
                print(f"  - {req_id}: No implementing task", file=sys.stderr)
            for task_id in orphan_tasks:
                print(f"  - {task_id}: No linked requirements", file=sys.stderr)
            return False
        
        return True


def main():
    """Main entry point."""
    import argparse
    
    parser = argparse.ArgumentParser(description='Display TDL traceability status')
    parser.add_argument('--gaps', action='store_true', help='Show only gaps')
    parser.add_argument('--check', action='store_true', help='CI mode: exit 1 if gaps found')
    args = parser.parse_args()
    
    # Find repository root
    repo_root = Path.cwd()
    while repo_root != repo_root.parent:
        if (repo_root / '.git').exists() or (repo_root / 'Cargo.toml').exists():
            break
        repo_root = repo_root.parent
    else:
        print("Error: Could not find repository root", file=sys.stderr)
        sys.exit(1)
    
    analyzer = TraceabilityAnalyzer(repo_root)
    
    if args.check:
        # CI mode
        if not analyzer.check_integrity():
            sys.exit(1)
        print("✓ Traceability check passed")
    else:
        # Display mode
        analyzer.print_status(gaps_only=args.gaps)


if __name__ == '__main__':
    main()
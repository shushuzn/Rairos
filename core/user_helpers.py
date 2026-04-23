"""
User-Friendly Error Messages and Helpers.

Provides clear, helpful error messages and suggestions for users.
Inspired by the "simplify complexity" philosophy.
"""
from typing import Optional, Dict, Any


class UserError(Exception):
    """Base user-friendly error."""
    
    def __init__(self, message: str, suggestion: Optional[str] = None):
        super().__init__(message)
        self.suggestion = suggestion
    
    def get_helpful_message(self) -> str:
        """Get error message with suggestion."""
        msg = f"❌ {super().__str__()}"
        if self.suggestion:
            msg += f"\n💡 建议: {self.suggestion}"
        return msg


class DatabaseError(UserError):
    """Database-related error."""
    
    def __init__(self, message: str, suggestion: Optional[str] = None):
        super().__init__(message, suggestion)
    
    @staticmethod
    def not_found(item: str, item_id: str) -> "DatabaseError":
        """Create a not found error."""
        return DatabaseError(
            message=f"未找到 {item}: {item_id}",
            suggestion=f"请检查 {item_id} 是否正确，或使用 'search' 命令搜索相关 {item}"
        )
    
    @staticmethod
    def connection_failed() -> "DatabaseError":
        """Create a connection failed error."""
        return DatabaseError(
            message="数据库连接失败",
            suggestion="请确保数据库文件存在，或运行 'python cli.py init' 初始化数据库"
        )


class APIError(UserError):
    """API-related error."""
    
    def __init__(self, message: str, suggestion: Optional[str] = None):
        super().__init__(message, suggestion)
    
    @staticmethod
    def rate_limit(endpoint: str, wait_seconds: int) -> "APIError":
        """Create a rate limit error."""
        return APIError(
            message=f"API请求过于频繁 ({endpoint})",
            suggestion=f"请等待 {wait_seconds} 秒后重试，或使用 'rate-limit' 命令查看API使用统计"
        )
    
    @staticmethod
    def network_failed() -> "APIError":
        """Create a network failed error."""
        return APIError(
            message="网络连接失败",
            suggestion="请检查网络连接，或使用代理设置"
        )
    
    @staticmethod
    def auth_failed() -> "APIError":
        """Create an authentication error."""
        return APIError(
            message="API认证失败",
            suggestion="请检查API密钥是否正确，或使用 'export OPENAI_API_KEY=your-key' 设置"
        )


class ParseError(UserError):
    """Paper parsing error."""
    
    def __init__(self, message: str, suggestion: Optional[str] = None):
        super().__init__(message, suggestion)
    
    @staticmethod
    def pdf_failed(paper_id: str) -> "ParseError":
        """Create a PDF parsing error."""
        return ParseError(
            message=f"解析论文失败: {paper_id}",
            suggestion="请检查PDF文件是否可访问，或使用 '--no-pdf' 跳过PDF下载"
        )


def format_error(error: Exception) -> str:
    """Format an error for user display."""
    if isinstance(error, UserError):
        return error.get_helpful_message()
    else:
        return f"❌ 发生错误: {error}"


def print_error(error: Exception):
    """Print error message to console."""
    print(format_error(error), file=__import__('sys').stderr)


class ProgressIndicator:
    """Simple progress indicator for long operations."""
    
    def __init__(self, total: int, description: str = "处理中"):
        self.total = total
        self.current = 0
        self.description = description
    
    def update(self, increment: int = 1):
        """Update progress."""
        self.current += increment
        percentage = (self.current / self.total * 100) if self.total > 0 else 0
        print(f"\r{self.description}: {self.current}/{self.total} ({percentage:.0f}%)", end="")
    
    def finish(self):
        """Finish progress indicator."""
        print(f"\r{self.description}: 完成！{self.current}/{self.total}")
    
    def __enter__(self):
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.finish()


def confirm_action(prompt: str) -> bool:
    """Ask user for confirmation."""
    response = input(f"{prompt} (y/n): ").strip().lower()
    return response in ['y', 'yes']


def select_option(options: list, prompt: str = "请选择") -> int:
    """Let user select an option."""
    print(f"\n{prompt}:")
    for i, option in enumerate(options, 1):
        print(f"  {i}. {option}")
    
    while True:
        try:
            choice = int(input("\n请输入选项编号: ").strip())
            if 1 <= choice <= len(options):
                return choice - 1
            print(f"请输入 1-{len(options)} 之间的数字")
        except ValueError:
            print("请输入有效的数字")


def print_table(headers: list, rows: list, max_width: int = 50):
    """Print data as a formatted table."""
    # Calculate column widths
    col_widths = [len(h) for h in headers]
    for row in rows:
        for i, cell in enumerate(row):
            col_widths[i] = max(col_widths[i], min(len(str(cell)), max_width))
    
    # Print headers
    header_line = " | ".join(h.ljust(col_widths[i]) for i, h in enumerate(headers))
    print(header_line)
    print("-" * len(header_line))
    
    # Print rows
    for row in rows:
        row_line = " | ".join(str(cell).ljust(col_widths[i])[:col_widths[i]] for i, cell in enumerate(row))
        print(row_line)


def print_json(data: Dict[str, Any], indent: int = 2):
    """Print data as formatted JSON."""
    import json
    print(json.dumps(data, indent=indent, ensure_ascii=False))


def print_banner(text: str, width: int = 60):
    """Print a banner."""
    padding = (width - len(text) - 2) // 2
    print("=" * width)
    print(" " * padding + text)
    print("=" * width)

"""
System info skill — quick system status checks.
"""

import platform
import shutil
import psutil
import logging
from datetime import datetime
from amanclaw.skills import skill

logger = logging.getLogger("amanclaw.skills.system_info")


@skill(
    name="system_status",
    description="Get current system status: CPU, memory, disk usage, uptime. Use when the user asks about system health or resource usage.",
    parameters={},
    timeout=10,
)
def system_status() -> str:
    """Get system status overview."""
    cpu_percent = psutil.cpu_percent(interval=1)
    mem = psutil.virtual_memory()
    disk = shutil.disk_usage("/")

    boot_time = datetime.fromtimestamp(psutil.boot_time())
    uptime = datetime.now() - boot_time

    return (
        f"System: {platform.system()} {platform.release()}\n"
        f"CPU: {cpu_percent}% ({psutil.cpu_count()} cores)\n"
        f"Memory: {mem.percent}% used ({_fmt(mem.used)} / {_fmt(mem.total)})\n"
        f"Disk: {disk.used / disk.total * 100:.1f}% used ({_fmt(disk.used)} / {_fmt(disk.total)})\n"
        f"Uptime: {uptime.days}d {uptime.seconds // 3600}h {(uptime.seconds % 3600) // 60}m"
    )


def _fmt(bytes_val: int) -> str:
    for unit in ("B", "KB", "MB", "GB"):
        if bytes_val < 1024:
            return f"{bytes_val:.1f}{unit}"
        bytes_val /= 1024
    return f"{bytes_val:.1f}TB"

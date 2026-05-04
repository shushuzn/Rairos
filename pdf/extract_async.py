"""Async PDF download using aiohttp."""
from pathlib import Path
from typing import Optional

import aiohttp

_connector = aiohttp.TCPConnector(limit=10, keepalive_timeout=30)
_session: Optional[aiohttp.ClientSession] = None


async def _get_session() -> aiohttp.ClientSession:
    global _session
    if _session is None or _session.closed:
        _session = aiohttp.ClientSession(connector=_connector)
    return _session


async def download_pdf_async(
    pdf_url: str,
    out_path: Path,
    timeout: int = 60,
    resume_size: int = 0,
) -> None:
    """Download PDF with resume support via aiohttp. Overwrites out_path on success."""
    out_path.parent.mkdir(parents=True, exist_ok=True)
    resume_path = out_path.with_suffix(".part")
    headers: dict[str, str] = {}
    if resume_size > 0:
        headers["Range"] = f"bytes={resume_size}-"

    timeout_cfg = aiohttp.ClientTimeout(total=timeout)
    session = await _get_session()

    async with session.get(pdf_url, headers=headers, raise_for_status=True, timeout=timeout_cfg) as r:
        supports_range = r.status == 206 or (
            resume_size > 0 and r.headers.get("Accept-Ranges", "none") != "none"
        )
        if supports_range and resume_size > 0:
            # Resume: append to existing partial file
            with open(resume_path, "ab") as f:
                async for chunk in r.content.iter_chunked(1024 * 1024):
                    if chunk:
                        f.write(chunk)
        else:
            # No resume support or no partial file: overwrite
            resume_path.unlink(missing_ok=True)
            with open(out_path, "wb") as f:
                async for chunk in r.content.iter_chunked(1024 * 1024):
                    if chunk:
                        f.write(chunk)
            return

    # Finalize: rename .part → target
    if resume_path.exists() and resume_path.stat().st_size > 0:
        resume_path.rename(out_path)
    elif not out_path.exists():
        raise RuntimeError(f"Download failed for {pdf_url}: no content received")

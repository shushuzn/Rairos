"""LLM API client."""
import json
import os
from typing import Dict, List, Optional

import requests


def call_llm_chat_completions(
    messages: List[Dict[str, str]],
    model: str,
    user_prompt: Optional[str] = None,
    base_url: str = "https://api.openai.com/v1",
    api_key: Optional[str] = None,
    timeout: int = 180,
    system_prompt: Optional[str] = None,
) -> str:
    api_key = api_key or os.getenv("OPENAI_API_KEY", "")
    if not api_key:
        raise ValueError("Missing API key. Provide --api-key or set OPENAI_API_KEY.")

    url = base_url.rstrip("/") + "/chat/completions"
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
    }
    msgs = list(messages)
    if system_prompt:
        msgs = [{"role": "system", "content": system_prompt}] + msgs
    payload = {
        "model": model,
        "temperature": 0.2,
        "messages": msgs,
    }
    if user_prompt:
        payload["messages"] = msgs + [{"role": "user", "content": user_prompt}]

    r = requests.post(url, headers=headers, data=json.dumps(payload), timeout=timeout)
    r.raise_for_status()
    data = r.json()
    return data["choices"][0]["message"]["content"]

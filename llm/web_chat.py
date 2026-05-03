"""Web Chat — FastAPI routes for streaming RAG chat over the paper library."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, AsyncIterator, Dict, List

from fastapi import APIRouter, Request
from fastapi.responses import JSONResponse, StreamingResponse

router = APIRouter()

SESSION_FILE = Path.home() / ".ai_research_os" / "chat_sessions.json"
SESSION_FILE.parent.mkdir(parents=True, exist_ok=True)


def _load_sessions() -> Dict[str, Any]:
    if not SESSION_FILE.exists():
        return {}
    return json.loads(SESSION_FILE.read_text(encoding="utf-8"))


def _save_sessions(sessions: Dict[str, Any]) -> None:
    SESSION_FILE.write_text(json.dumps(sessions, indent=2, ensure_ascii=False), encoding="utf-8")


async def chat_stream(request: Request) -> StreamingResponse:
    """Streaming chat endpoint — POST JSON {query}, returns SSE."""
    try:
        body = await request.json()
        query = body.get("query", "").strip()
        session_id = body.get("session_id", "default")
    except Exception:
        query = ""

    if not query:
        return StreamingResponse(iter([f"data: {json.dumps({'error': 'empty query'})}\n\n"]), media_type="text/event-stream")

    contexts: List[Dict[str, Any]] = []
    try:
        from llm.chat import RAGChat
        rag = RAGChat()
        results = rag.answer(query, use_llm=False, concept=None, top_k=5)
        contexts = results.get("retrieved_contexts", [])[:5]
    except Exception:
        pass

    async def event_stream() -> AsyncIterator[str]:
        yield f"data: {json.dumps({'type': 'context', 'count': len(contexts)})}\n\n"

        full_response = ""
        try:
            from llm.chat import stream_llm_chat_completions
            from llm.client import build_rag_prompt

            system_prompt = (
                "You are an expert research assistant. Answer questions about the user's paper library. "
                "Use the provided context snippets to ground your answers."
            )
            context_text = "\n\n".join(
                f"[Paper: {c.get('title', 'Unknown')}]\n{c.get('chunk', c.get('text', ''))}"
                for c in contexts
            )
            user_prompt = build_rag_prompt(query, context_text)
            messages = [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ]
            async for chunk in stream_llm_chat_completions(messages, model=None, base_url=None, api_key=None):
                full_response += chunk
                yield f"data: {json.dumps({'type': 'chunk', 'content': chunk})}\n\n"
        except Exception as e:
            yield f"data: {json.dumps({'type': 'error', 'content': str(e)})}\n\n"

        # Save to session
        sessions = _load_sessions()
        if session_id not in sessions:
            sessions[session_id] = {"messages": []}
        sessions[session_id]["messages"].append({"role": "user", "content": query})
        sessions[session_id]["messages"].append({"role": "assistant", "content": full_response})
        sessions[session_id]["updated_at"] = str(__import__("datetime").datetime.now().isoformat())
        _save_sessions(sessions)

        yield f"data: {json.dumps({'type': 'done'})}\n\n"

    return StreamingResponse(event_stream(), media_type="text/event-stream")


def render_chat_html() -> str:
    lines = ['<div class="web-chat">']
    lines.append("<h3>💬 Research Chat</h3>")
    lines.append("<p style='font-size:13px;color:#A89E8C;margin-bottom:14px'>"
                "Ask questions about your paper library. Answers are grounded in your papers.</p>")

    lines.append("<div id='chatMessages' style='max-height:400px;overflow-y:auto;"
                "border:1px solid #e0dbd4;border-radius:6px;padding:12px;margin-bottom:12px;"
                "background:#faf8f5'>")
    lines.append("<p id='emptyHint' style='text-align:center;color:#A89E8C;font-size:13px;padding:20px;margin:0'>"
                "Ask your first question below</p>")
    lines.append("</div>")

    lines.append("<div style='display:flex;gap:8px'>")
    lines.append("<input type='text' id='chatInput' placeholder='Ask about your papers...' "
                "style='flex:1;font-size:13px;padding:8px 12px;border:1px solid #ccc;"
                "border-radius:4px;font-family:Georgia,serif' "
                "onkeydown='if(event.key===\"Enter\")sendChat()'>")
    lines.append("<button id='sendBtn' "
                "style='background:#6B8FB5;color:white;border:none;border-radius:4px;"
                "padding:8px 16px;cursor:pointer;font-size:13px'>Send</button>")
    lines.append("</div>")
    lines.append("<p id='chatStatus' style='font-size:11px;color:#A89E8C;margin-top:6px;margin-bottom:0;height:16px'></p>")

    lines.append("""
<script>
(function() {
    var sessionId = 'default';
    var chatContainer = document.getElementById('chatMessages');
    var emptyHint = document.getElementById('emptyHint');
    var input = document.getElementById('chatInput');
    var sendBtn = document.getElementById('sendBtn');
    var statusEl = document.getElementById('chatStatus');

    function createMsgDiv(role, text) {
        var div = document.createElement('div');
        div.style.marginBottom = '10px';
        div.style.padding = '8px 10px';
        div.style.borderRadius = '6px';
        var label = document.createElement('span');
        label.style.fontSize = '11px';
        label.style.fontWeight = '700';
        if (role === 'user') {
            div.style.background = 'rgba(107,143,181,0.12)';
            div.style.textAlign = 'right';
            label.style.color = '#6B8FB5';
            label.textContent = 'You';
        } else {
            div.style.background = '#f0ebe5';
            label.style.color = '#A89E8C';
            label.textContent = 'Rairos';
        }
        var content = document.createElement('span');
        content.style.fontSize = '13px';
        content.style.color = '#2a2a2a';
        content.style.whiteSpace = 'pre-wrap';
        content.textContent = text;
        div.appendChild(label);
        div.appendChild(document.createElement('br'));
        div.appendChild(content);
        return div;
    }

    function sendChat() {
        var query = input.value.trim();
        if (!query) return;
        if (emptyHint) emptyHint.style.display = 'none';
        chatContainer.appendChild(createMsgDiv('user', query));
        input.value = '';
        sendBtn.disabled = true;
        statusEl.textContent = 'Thinking...';

        var assistantDiv = createMsgDiv('assistant', '');
        chatContainer.appendChild(assistantDiv);
        var assistantContent = assistantDiv.querySelector('span:last-child');

        fetch('/chat/stream', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({query: query, session_id: sessionId})
        }).then(function(r) {
            var reader = r.body.getReader();
            var decoder = new TextDecoder();
            var fullText = '';
            function read() {
                reader.read().then(function(result) {
                    if (result.done) return;
                    var text = decoder.decode(result.value);
                    var lines = text.split('\\n');
                    for (var i = 0; i < lines.length; i++) {
                        var ln = lines[i].trim();
                        if (!ln.startsWith('data: ')) continue;
                        try {
                            var data = JSON.parse(ln.slice(6));
                            if (data.type === 'chunk') {
                                fullText += data.content;
                                assistantContent.textContent = fullText;
                                chatContainer.scrollTop = chatContainer.scrollHeight;
                            } else if (data.type === 'done') {
                                sendBtn.disabled = false;
                                statusEl.textContent = '';
                            }
                        } catch(e) {}
                    }
                    read();
                });
            }
            read();
        }).catch(function(err) {
            assistantContent.textContent = 'Error: ' + err.message;
            sendBtn.disabled = false;
            statusEl.textContent = '';
        });
    }

    sendBtn.addEventListener('click', sendChat);
})();
</script>""")

    lines.append("<style>.web-chat { font-family: Georgia, serif; }</style>")
    lines.append("</div>")
    return "\n".join(lines)

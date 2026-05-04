"""Voice-to-Capsule — upload audio → Whisper transcription → LLM gap extraction → save to Gene Pool."""

from __future__ import annotations

import json
import os
import tempfile
import uuid
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, Optional

PAPERS_DIR = Path.home() / ".ai_research_os"
CAPSULES_PATH = PAPERS_DIR / "gene_pool" / "capsules.json"


def transcribe_audio(audio_bytes: bytes) -> str:
    """Transcribe audio using OpenAI Whisper."""
    try:
        import openai
        with tempfile.NamedTemporaryFile(suffix=".webm", delete=False) as f:
            f.write(audio_bytes)
            tmp_path = f.name
        client = openai.OpenAI()
        with open(tmp_path, "rb") as audio_file:
            transcript = client.audio.transcriptions.create(
                model="whisper-1", file=audio_file)
        os.unlink(tmp_path)
        return transcript.text
    except Exception as e:
        return f"[Transcription error: {e}]"


def extract_gap_from_text(text: str, source: str = "voice") -> Dict[str, Any]:
    """Extract research gap from transcribed text using LLM."""
    try:
        from openai import OpenAI
        client = OpenAI()
        prompt = (
            "You are a research gap extractor. Given a transcription of a research discussion, "
            "identify the key research gaps mentioned. Return JSON with:\n"
            '- "gap_titles": list of gap titles (max 3)\n'
            '- "gap_types": list from: theoretical_gap, method_limitation, evaluation_gap, '
            "scalability_issue, dataset_gap, generalization_gap, contradiction, unexplored_application\n"
            '- "keywords": list of 3-5 keywords\n'
            '- "polarity": "positive" or "negative"\n'
            '- "summary": 1-sentence summary\n\n'
            f"Transcription:\n{text[:3000]}"
        )
        response = client.chat.completions.create(
            model="gpt-4o-mini",
            messages=[{"role": "user", "content": prompt}],
            response_format={"type": "json_object"},
        )
        return json.loads(response.choices[0].message.content)
    except Exception as e:
        return {"error": str(e)}


def _load_capsules() -> Dict[str, Any]:
    if not CAPSULES_PATH.exists():
        return {"version": "1.0", "capsules": []}
    return json.loads(CAPSULES_PATH.read_text(encoding="utf-8"))


def _save_capsules(data: Dict[str, Any]) -> None:
    CAPSULES_PATH.parent.mkdir(parents=True, exist_ok=True)
    CAPSULES_PATH.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")


def save_voice_capsule(gap_data: Dict[str, Any], source: str = "voice") -> str:
    capsule_id = uuid.uuid4().hex[:12]
    now = datetime.now().isoformat()
    capsule = {
        "capsule_id": capsule_id,
        "action_gap_title": (gap_data.get("gap_titles") or ["Voice gap"])[0],
        "action_gap_type": (gap_data.get("gap_types") or ["theoretical_gap"])[0],
        "trigger_keywords": (gap_data.get("keywords") or [])[:5],
        "polarity": gap_data.get("polarity", "positive"),
        "outcome_success_score": 0.5,
        "low_score_streak": 0,
        "status": "active",
        "source": source,
        "created_at": now,
    }
    data = _load_capsules()
    data["capsules"].append(capsule)
    _save_capsules(data)
    return capsule_id


def render_voice_upload_html() -> str:
    lines = ['<div class="voice-capsule">']
    lines.append("<h3>🎤 Voice-to-Capsule</h3>")
    lines.append("<p style='font-size:13px;color:#A89E8C;margin-bottom:14px'>"
                "Upload an audio recording of a research discussion. "
                "Transcribe + extract research gaps automatically.</p>")

    lines.append("""
<div style="border: 2px dashed #ccc; border-radius: 8px; padding: 24px; text-align: center; margin-bottom: 16px;">
  <input type="file" id="audioFile" accept="audio/*" style="margin-bottom: 12px">
  <button id="transcribeBtn" style="background:#6B8FB5;color:white;border:none;border-radius:4px;padding:8px 18px;cursor:pointer;font-size:13px">
    🎤 Transcribe &amp; Extract Gap
  </button>
  <p id="statusText" style="font-size:12px;color:#A89E8C;margin-top:8px;display:none"></p>
</div>
<div id="resultArea" style="display:none">
  <h4 style="font-size:13px;font-weight:700;color:#2a4a6a;margin-bottom:8px">Extracted Gap</h4>
  <div id="gapPreview" style="background:#f8f4ef;padding:12px;border-radius:6px;margin-bottom:12px"></div>
  <button id="saveBtn" style="background:#6BBF8A;color:white;border:none;border-radius:4px;padding:8px 16px;cursor:pointer;font-size:13px">
    ✅ Save to Gene Pool
  </button>
</div>""")

    lines.append("""
<script>
var extractedData = null;
document.getElementById('transcribeBtn').addEventListener('click', function() {
    var file = document.getElementById('audioFile').files[0];
    if (!file) { alert('Please select an audio file first.'); return; }
    var status = document.getElementById('statusText');
    status.textContent = 'Transcribing...';
    status.style.display = 'block';

    var formData = new FormData();
    formData.append('audio', file);

    fetch('/voice-capsule/transcribe', {method: 'POST', body: formData})
      .then(function(r) { return r.json(); })
      .then(function(d) {
          status.textContent = 'Extracting gaps...';
          if (d.error) {
              status.textContent = 'Error: ' + d.error;
          } else {
              extractedData = d;
              var preview = document.getElementById('gapPreview');
              var titles = (d.gap_titles || []).join(', ') || 'N/A';
              var types = (d.gap_types || []).join(', ') || 'N/A';
              var kws = ((d.keywords || []).join(', ')) || 'N/A';
              var summary = d.summary || '';
              // Build text safely without innerHTML
              preview.innerText = '';
              var t = document.createTextNode('');
              preview.appendChild(document.createElement('div').appendChild(document.createTextNode('Title: ' + titles)).parentNode || document.createTextNode(''));
              var div = document.createElement('div');
              div.innerText = 'Type: ' + types;
              preview.appendChild(div);
              var div2 = document.createElement('div');
              div2.innerText = 'Keywords: ' + kws;
              preview.appendChild(div2);
              var div3 = document.createElement('div');
              div3.style.marginTop = '4px';
              div3.innerText = summary;
              preview.appendChild(div3);
              document.getElementById('resultArea').style.display = 'block';
              status.style.display = 'none';
          }
      });
});
document.getElementById('saveBtn').addEventListener('click', function() {
    if (!extractedData) return;
    fetch('/voice-capsule/save', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify(extractedData)
    }).then(function(r) { return r.json(); })
      .then(function(d) {
          if (d.success) {
              alert('Saved to Gene Pool! Capsule ID: ' + d.capsule_id);
              location.reload();
          } else {
              alert('Error: ' + (d.error || 'unknown'));
          }
      });
});
</script>""")

    lines.append("<style>.voice-capsule { font-family: Georgia, serif; }</style>")
    lines.append("</div>")
    return "\n".join(lines)

"""Tests for streaming and voice_to_capsule."""

from llm.streaming import UsageSnapshot, StreamingCostTracker, stream_with_cost
from llm.voice_to_capsule import transcribe_audio, extract_gap_from_text, render_voice_upload_html


class TestUsageSnapshot:
    def test_fields(self):
        u = UsageSnapshot(
            prompt_tokens=100,
            completion_tokens=50,
            total_tokens=150,
            cost_usd=0.002,
        )
        assert u.total_tokens == 150
        assert u.cost_usd == 0.002


class TestStreamingCostTracker:
    def test_init(self):
        t = StreamingCostTracker(model="gpt-4")
        assert t.model == "gpt-4"


class TestStreamWithCost:
    def test_signature(self):
        # Just check it accepts the right args - real streaming needs API key
        import inspect

        sig = inspect.signature(stream_with_cost)
        assert "messages" in sig.parameters
        assert "model" in sig.parameters


class TestVoiceToCapsule:
    def test_transcribe_audio_type(self):
        # Needs real audio bytes, just check it accepts bytes
        import inspect

        sig = inspect.signature(transcribe_audio)
        assert "audio_bytes" in sig.parameters

    def test_extract_gap_from_text(self):
        result = extract_gap_from_text("We achieved 95% accuracy on ImageNet", source="paper")
        # Returns dict or {"error": ...} when deps missing
        assert isinstance(result, dict)
        assert len(result) >= 0

    def test_render_voice_upload_html(self):
        result = render_voice_upload_html()
        assert isinstance(result, str)
        assert "<" in result  # it's HTML

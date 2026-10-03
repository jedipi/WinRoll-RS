import json
import subprocess
from pathlib import Path
from faster_whisper import WhisperModel
import imageio_ffmpeg
import soundfile as sf

model = WhisperModel("base.en", device="cpu", compute_type="int8", cpu_threads=4, download_root=".hyperframes/models")
subprocess.run([imageio_ffmpeg.get_ffmpeg_exe(), "-y", "-i", "narration.wav", "-ar", "16000", "-ac", "1", ".hyperframes/timing.wav"], check=True, capture_output=True)
audio, sample_rate = sf.read(".hyperframes/timing.wav", dtype="float32")
assert sample_rate == 16000
segments, info = model.transcribe(audio, language="en", word_timestamps=True, beam_size=5)
words = []
for segment in segments:
    for word in segment.words or []:
        words.append({"text": word.word.strip(), "start": round(word.start, 3), "end": round(word.end, 3)})
Path("transcript-raw.json").write_text(json.dumps({"words": words, "duration": info.duration}, indent=2), encoding="utf-8")
assert words and all(word["end"] >= word["start"] for word in words)
print(json.dumps({"duration": info.duration, "text": " ".join(w["text"] for w in words)}))

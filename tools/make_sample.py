"""Create a locally generated tone for decoder/UI checks. Not YouTube playback."""
import math
from pathlib import Path
import struct
import wave

path = Path(__file__).resolve().parent / "fixtures" / "tone.wav"
path.parent.mkdir(exist_ok=True)
rate = 48000
with wave.open(str(path), "wb") as output:
    output.setnchannels(1)
    output.setsampwidth(2)
    output.setframerate(rate)
    # Quiet, bounded 30-second sample. No network or copyrighted media required.
    for second in range(30):
        output.writeframes(b"".join(struct.pack("<h", int(1200 * math.sin(2 * math.pi * 220 * (second * rate + i) / rate))) for i in range(rate)))
print(path)

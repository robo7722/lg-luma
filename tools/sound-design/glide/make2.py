import numpy as np, wave, os
W = os.path.dirname(os.path.abspath(__file__))
R = 48000
rng = np.random.default_rng(7)
src = np.frombuffer(open(f'{W}/tick.raw','rb').read(), dtype='<i2').astype(np.float32) / 32768
tick = src[: int(0.040 * R)]

def rate(x, r):
    n = int(len(x) / r)
    return np.interp(np.arange(n) * r, np.arange(len(x)), x)

def rms_above(x):
    m = np.abs(x) > 0.05 * np.abs(x).max()
    return np.sqrt(np.mean(x[m] ** 2))

with wave.open('/home/user/lg-luma/assets/sounds/focus.wav') as w:
    ref = rms_above(np.frombuffer(w.readframes(w.getnframes()), dtype='<i2').astype(np.float32) / 32768)

def finish(x, factor):
    x = x / (np.abs(x).max() + 1e-9) * 0.5
    fi = int(0.002 * R); fo = int(0.015 * R)
    x[:fi] *= np.linspace(0, 1, fi); x[-fo:] *= np.linspace(1, 0, fo)
    x = x * (ref * factor / rms_above(x))
    return np.tanh(1.2 * x) / np.tanh(1.2)

def save(name, x):
    with wave.open(f'{W}/{name}.wav', 'wb') as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(R)
        w.writeframes((np.clip(x, -1, 1) * 32767).astype('<i2').tobytes())

# B: airy whoosh – band-passed noise whose centre sweeps 700 Hz -> 3.2 kHz, soft swell and fade
n = int(0.20 * R); t = np.arange(n) / R
noise = rng.standard_normal(n)
out = np.zeros(n); y1 = y2 = 0.; x1 = x2 = 0.
for i in range(n):
    f = 700 * (3200 / 700) ** (t[i] / t[-1]); q = 1.6
    w0 = 2 * np.pi * f / R; alpha = np.sin(w0) / (2 * q)
    b0, b2 = alpha, -alpha; a0 = 1 + alpha; a1 = -2 * np.cos(w0); a2 = 1 - alpha
    v = (b0 * noise[i] + b2 * x2 - a1 * y1 - a2 * y2) / a0
    x2, x1 = x1, noise[i]; y2, y1 = y1, v; out[i] = v
env = np.sin(np.pi * np.clip(t / t[-1], 0, 1)) ** 1.5
save('glide-b', finish(out * env, 0.9))

# D: glass ping – the tick pitched up, ringing briefly through a tuned comb (like a tapped glass)
n = int(0.22 * R)
x = np.zeros(n); tk = rate(tick, 1.5); x[:len(tk)] = tk
delay = int(R / 2093)  # C7
y = np.copy(x)
for i in range(delay, n):
    y[i] += 0.93 * y[i - delay]
decay = np.exp(-np.arange(n) / (0.06 * R))
save('glide-d', finish(y * decay, 1.0))
print('ok')

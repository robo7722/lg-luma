import numpy as np, wave, sys, os
W = os.path.dirname(os.path.abspath(__file__))
R = 48000
src = np.frombuffer(open(f'{W}/tick.raw','rb').read(), dtype='<i2').astype(np.float32) / 32768
tick = src[: int(0.040 * R)]

def rate(x, r):
    # playbackRate: resample by linear interpolation
    n = int(len(x) / r)
    return np.interp(np.arange(n) * r, np.arange(len(x)), x)

def lowpass(x, hz):
    # one-pole lowpass, applied twice
    a = np.exp(-2 * np.pi * hz / R)
    for _ in range(2):
        y = np.zeros_like(x); s = 0.
        for i, v in enumerate(x):
            s = (1 - a) * v + a * s; y[i] = s
        x = y
    return x

def mix(length, layers):
    out = np.zeros(int(length * R))
    for at, r, gain, lp in layers:
        t = rate(tick, r)
        if lp: t = lowpass(t, lp)
        i = int(at * R); n = min(len(t), len(out) - i)
        out[i:i + n] += gain * t[:n]
    return out

def finish(x, peak):
    x = x / (np.abs(x).max() + 1e-9) * peak
    fi = int(0.001 * R); fo = int(0.010 * R)
    x[:fi] *= np.linspace(0, 1, fi); x[-fo:] *= np.linspace(1, 0, fo)
    return x

focus = np.frombuffer(open(f'{W}/../../../../../../../home/user/lg-luma/assets/sounds/focus.wav','rb').read()[44:], dtype='<i2').astype(np.float32)/32768 if False else None

def rms_above(x):
    m = np.abs(x) > 0.05 * np.abs(x).max()
    return np.sqrt(np.mean(x[m] ** 2))

def loud(x, ref_rms, factor):
    x = x * (ref_rms * factor / rms_above(x))
    return np.tanh(1.2 * x) / np.tanh(1.2)

def save(name, x):
    data = (np.clip(x, -1, 1) * 32767).astype('<i2').tobytes()
    with wave.open(f'{W}/{name}.wav', 'wb') as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(R); w.writeframes(data)

with wave.open('/home/user/lg-luma/assets/sounds/focus.wav') as w:
    f = np.frombuffer(w.readframes(w.getnframes()), dtype='<i2').astype(np.float32) / 32768
ref = rms_above(f)

# A: glass shimmer – three soft ticks rising in pitch, softened
a = mix(0.150, [(0.000, 0.85, 0.9, 3500), (0.035, 1.00, 0.7, 4000), (0.070, 1.18, 0.5, 4500)])
# B: airy slide – four quick, very soft ticks rising, gentle lowpass (like a sheet gliding)
b = mix(0.160, [(0.000, 0.75, 0.6, 2500), (0.028, 0.90, 0.6, 3000), (0.056, 1.05, 0.5, 3500), (0.084, 1.20, 0.35, 4000)])
# C: single soft "pop" – one tick pitched down with a short echo, very muted
c = mix(0.130, [(0.000, 0.70, 1.0, 2200), (0.050, 0.70, 0.35, 1600)])
for name, x in [('glide-a', a), ('glide-b', b), ('glide-c', c)]:
    x = finish(x, 0.5)
    # A touch quieter than focus: these play with motion, not on every key.
    save(name, loud(x, ref, 1.1))
print('ok')

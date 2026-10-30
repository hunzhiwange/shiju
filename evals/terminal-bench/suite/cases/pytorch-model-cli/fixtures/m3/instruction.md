# m3: Synthetic model CLI diagnostic

Work only in /app. Create solution.py using Python 3 standard library.

Implement `python3 solution.py weights.json image.png`. Valid weights have exactly weight (four numeric values) and bias (one number). Decode 2x2 8-bit grayscale, noninterlaced filter-zero PNG, normalize pixels by 255 in row-major order, output the scalar dot(weight,pixels)+bias as JSON. On wrong argument count, nonexistent files, invalid JSON, missing/wrong-sized weights, non-PNG or wrong dimensions: exit 2, stdout empty, stderr nonempty. Valid input exits 0 with empty stderr. Tests include intermediate pixel intensities.

No network, host files, or external services. Limit: 24 model turns, 32 tool calls, 900 seconds; one attempt.

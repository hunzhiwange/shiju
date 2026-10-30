# m1: Synthetic model CLI diagnostic

Work only in /app. Create solution.py using Python 3 standard library.

Read weights.json and image.png using `python3 solution.py weights.json image.png`. Output one JSON object with shape [2,2], pixels (four grayscale values divided by 255 in row-major order), and counts (length of each flat weight array). Supplied and hidden images are 2x2, 8-bit grayscale, noninterlaced PNGs with filter 0; validate those dimensions. Weight keys are w1,b1,w2,b2. No model inference is required.

No network, host files, or external services. Limit: 24 model turns, 32 tool calls, 900 seconds; one attempt.

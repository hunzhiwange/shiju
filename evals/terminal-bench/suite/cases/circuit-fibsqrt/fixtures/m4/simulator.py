import sys
# A fixed eight-wire NOT/copy simulator, with little-endian ports.
bits = [(int(sys.argv[1]) >> i) & 1 for i in range(8)]
result = [1-b if i in (0, 2, 5, 7) else b for i, b in enumerate(bits)]
print(sum(b * 2**i for i,b in enumerate(result)))

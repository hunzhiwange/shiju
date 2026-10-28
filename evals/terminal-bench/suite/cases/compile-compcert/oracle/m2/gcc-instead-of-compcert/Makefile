all: ccomp

ccomp: driver.c
	cc -DCOMPILER=\"/usr/bin/gcc\" -O2 -Wall -Wextra driver.c -o ccomp

clean:
	rm -f ccomp

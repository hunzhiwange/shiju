all: ccomp

ccomp: driver.c
	cc -O2 -Wall -Wextra driver.c -lmissing_compcert_runtime -o ccomp

clean:
	rm -f ccomp

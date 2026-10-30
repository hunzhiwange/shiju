import json, pathlib, re, subprocess, sys, tempfile
request=json.load(sys.stdin)
def compile_one(item):
    with tempfile.TemporaryDirectory() as directory:
        work=pathlib.Path(directory)
        (work/'main.tex').write_text(item['main'])
        (work/'input.tex').write_text(item['input'])
        process=subprocess.run(['pdflatex','-no-shell-escape','-interaction=nonstopmode','-halt-on-error','main.tex'],cwd=work,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=15)
        log=(work/'main.log').read_text(errors='replace') if (work/'main.log').exists() else ''
        pdf=work/'main.pdf'
        return {'compiled':process.returncode==0,'overfull':0, 'pdf':pdf.exists() and pdf.read_bytes().startswith(b'%PDF-')}
print(json.dumps(compile_one(request)))

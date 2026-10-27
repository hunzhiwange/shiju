from pathlib import Path
for n,v in [("helpers",11),("complexity",23),("invariants",37)]:
    Path("ribbon/"+n+".py").write_text("def identity(): return "+str(v))

import json, sys

def main():
    d=json.load(sys.stdin)
    v=d['values']
    print(json.dumps(dict(label='station',half=len(v)//2,mean=sum(v)/len(v) if v else None)))
if __name__=='__main__':
    main()

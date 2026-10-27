import sys,grpc
from concurrent.futures import ThreadPoolExecutor
import kv_store_pb2 as p
import kv_store_pb2_grpc as rpc
class Server(rpc.KVStoreServicer):
    def __init__(self): self.data={}
    def SetVal(self,request,context):
        self.data[request.key]=request.value
        return p.SetValResponse(val=request.value)
    def GetVal(self,request,context):
        return p.GetValResponse(val=self.data.get(request.key,0))
s=grpc.server(ThreadPoolExecutor(max_workers=2))
rpc.add_KVStoreServicer_to_server(Server(),s)
s.add_insecure_port('127.0.0.1:'+sys.argv[1])
s.start()
s.wait_for_termination()

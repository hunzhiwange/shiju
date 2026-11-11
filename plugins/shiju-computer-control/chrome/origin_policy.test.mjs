import test from 'node:test';
import assert from 'node:assert/strict';
import { fixture } from './background.test.mjs';

test('cc_control_10_navigation_updates_only_bound_tab_and_invalidates_handle', async () => {
  const f = fixture(), target = await f.setup();
  const updates = [];
  f.api.tabs.update = async (id, args) => { updates.push({id,...args}); return {id}; };
  assert.equal((await f.request('navigate', {target,args:{url:'https://destination.test/'}})).ok, true);
  assert.deepEqual(updates,[{id:7,url:'https://destination.test/'}]);
  assert.equal((await f.request('read',{target,args:{}})).error,'stale_target');
});
test('cc_control_10_file_transfer_requires_host_approval_marker', async () => {
  for (const action of ['upload','download']) {
    const f = fixture(), target = await f.setup();
    assert.equal((await f.request(action,{target,args:{selector:'#file'}})).error,'transfer_not_approved');
    assert.equal(f.scripts.length,1);
  }
});
test('cc_control_10_approved_upload_targets_only_current_top_document', async () => {
  const f=fixture(),target=await f.setup();
  f.api.scripting.executeScript=async args=>{
    f.scripts.push(args);
    return [{documentId:target.documentId,frameId:0,result:{uploaded:true}}];
  };
  assert.equal((await f.request('upload',{target,args:{selector:'#file',fileName:'fixture.txt',contentBase64:'eA==',approved:true}})).ok,true);
  assert.deepEqual(f.scripts.at(-1).target,{tabId:7,documentIds:['document-1']});
});
test('cc_control_10_subframe_result_is_rejected', async () => {
  const f=fixture(),target=await f.setup();
  f.api.scripting.executeScript=async()=>[{documentId:target.documentId,frameId:2,result:{text:'child'}}];
  assert.equal((await f.request('read',{target,args:{}})).error,'stale_target');
});

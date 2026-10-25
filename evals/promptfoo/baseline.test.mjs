import test from 'node:test';
import assert from 'node:assert/strict';
import { loadTasks, grade } from './baseline.mjs';
test('baseline_loads_only_fifteen_dev_tasks_and_does_not_assert_isolation', async()=>{
 const tasks=await loadTasks();assert.equal(tasks.length,15);
 assert.ok(tasks.every(t=>t.metadata.split==='dev'&&t.definitions.has(t.taskId)));
 const result=grade('',{providerResponse:{metadata:{evaluationProfile:'runtime-accuracy',oracle:{pass:true,checks:[{pass:true}]},terminal:{pass:true},budget:{pass:true},boundary:{pass:false,verified:false},toolCalls:[]}}});
 assert.equal(result.pass,true);assert.match(result.reason,/非隔离/);
});
import { prepare, cleanup } from './attempt-workspace.mjs';
import { createArtifactOracle } from './artifact-oracle.mjs';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
test('baseline_keeps_all_fifteen_independent_positive_and_negative_oracles',async()=>{
 for(const task of await loadTasks()){
  const work=await prepare(task),definition=task.definitions.get(task.taskId),oracle=createArtifactOracle(task.definitions);
  try{
   for(const[file,bytes]of Object.entries(definition.expectedFiles)){
    await mkdir(path.dirname(path.join(work.workRoot,file)),{recursive:true});await writeFile(path.join(work.workRoot,file),bytes);
   }
   assert.equal((await oracle({taskId:task.taskId,workRoot:work.workRoot,response:JSON.stringify(definition.expectedJson)})).pass,true);
   const first=Object.keys(definition.expectedFiles)[0];if(first)await writeFile(path.join(work.workRoot,first),'wrong');
   assert.equal((await oracle({taskId:task.taskId,workRoot:work.workRoot,response:'{}'})).pass,false);
  }finally{await cleanup(work);}
 }
});

// Existing Gateway, explicitly unisolated; independent task/result oracles remain unchanged.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseSuite, verify } from './freeze-accuracy.mjs';
import { validateManifest, manifestDigest } from './accuracy-manifest.mjs';
import { createArtifactOracle } from './artifact-oracle.mjs';
import { evaluate } from './assertions.mjs';
import Provider, { aggregateEvents } from './vibewindow-provider.mjs';
const root=path.dirname(fileURLToPath(import.meta.url)), results=path.join(root,'results/accuracy-baseline');
const sha=value=>createHash('sha256').update(JSON.stringify(value)).digest('hex');
export async function loadTasks(){
 const tasks=[];
 for(const category of ['facts','edit','recovery','state','finish']){
  const source=await readFile(path.join(root,`fixtures/runtime-accuracy/${category}/oracle/definitions.mjs`),'utf8');
  const cases=JSON.parse(source.match(/export const cases = (\[[\s\S]*?\n\])\.map\(/)[1]);
  for(const row of parseSuite(await readFile(path.join(root,`tests/accuracy-${category}.yaml`),'utf8')).filter(r=>r.metadata.split==='dev')){
   const c=cases.find(c=>c.taskId===row.metadata.taskId&&c.split==='dev');
   const fixtureRoot=path.join(root,`fixtures/runtime-accuracy/${category}/${c.taskId}`);
   const expectedFiles=category==='facts'?{}:['edit','state'].includes(category)?{[c.target]:Buffer.from(c.reference)}:
    Object.fromEntries(Object.entries(c.expectedFiles??c.referenceFiles).map(([p,v])=>[p,Buffer.from(v)]));
   const unchangedFiles=Object.fromEntries(await Promise.all(c.fixtureFiles.filter(p=>!Object.hasOwn(expectedFiles,p)).map(async p=>[p,await readFile(path.join(fixtureRoot,p))])));
   const definition={expectedFiles,unchangedFiles,allowExtraFiles:false,
    ...(category==='facts'?{expectedJson:c.reference}:{}),
    ...(category==='recovery'?{expectedFailure:{toolName:'bash',operation:c.operation,exitCode:2,recoveryOperation:c.recoveryOperation}}:{})};
   row.vars.task=row.vars.task.trimEnd().replaceAll('\n',' ');
   tasks.push({...row,category,taskId:c.taskId,fixtureRoot,fixtureFiles:c.fixtureFiles,definitions:new Map([[c.taskId,definition]])});
  }
 }
 if(tasks.length!==15)throw Error('expected fifteen dev tasks');return tasks;
}
export function grade(output,context){
 const metadata=context.providerResponse?.metadata??{};
 const original=evaluate(output,{...context,providerResponse:{...context.providerResponse,metadata:{...metadata,evaluationProfile:'runtime-accuracy'}}});
 const checks=original.componentResults.filter(c=>!Object.hasOwn(c.namedScores,'boundary_checks'));
 const pass=checks.every(c=>c.pass);
 return {pass,score:pass?1:0,reason:'非隔离：边界和后台回收未验证；'+(pass?'原结果/终态/预算/恢复断言通过':'原结果/终态/预算/恢复断言未全部通过'),componentResults:checks};
}
export default class BaselineProvider{
 id(){return 'runtime-accuracy-unisolated-token-plan';}
 async callApi(prompt,context){
  verify();
  const task=(await loadTasks()).find(t=>t.taskId===context.vars?.taskId);
  if(!task||task.vars.task!==prompt)throw Error('untrusted dev task');
  const manifest=JSON.parse(await readFile(path.join(results,'before-manifest.json')));
  const environment=JSON.parse(await readFile(path.join(results,'live-environment.json')));
  const health=await(await fetch('http://127.0.0.1:42617/v1/health')).json();
  if(health.build_id!==environment.gatewayBuildId)throw Error('Gateway identity changed; stop sampling');
  await mkdir(path.join(results,'attempts'),{recursive:true});
  let directory;
  for(let i=1;i<=3;i++){
   const candidate=path.join(results,'attempts',`${task.taskId}-${i}`);
   try{await mkdir(candidate);directory=candidate;break;}catch(e){if(e.code!=='EEXIST')throw e;}
  }
  if(!directory)throw Error('three attempts already reserved; no retries');
  for(const key of Object.keys(process.env))if(key.startsWith('VIBEWINDOW_EVAL_')||['VIBEWINDOW_PLATFORM_ID','VIBEWINDOW_COMPANY_ID'].includes(key))delete process.env[key];
  const provider=new Provider({config:{endpoint:environment.endpoint,model:environment.model,agent:'main',
   platformId:'10000',companyId:'100000000',evaluationProfile:'runtime-accuracy-unisolated',manifest,
   taskId:task.taskId,fixtureRoot:task.fixtureRoot,fixtureFiles:task.fixtureFiles,timeoutMs:900000,maxIterations:24,maxToolCalls:32}});
  const result=await provider.callApi(prompt,{vars:{turns:task.vars.turns}});
  const data=result.metadata??={};
  if(data.events){const partial=aggregateEvents(data.events.filter(e=>e.type!=='chat.error'));Object.assign(data,partial.metadata);result.sessionId??=partial.sessionId;}
  data.mode='unisolated-user-requested';data.backgroundSettlement='unverified';data.isolationVerified=false;
  data.boundary={pass:false,verified:false,reason:'user explicitly requested no isolation'};
  data.requiresRecovery=task.category==='recovery';data.expectedFailure=task.definitions.get(task.taskId).expectedFailure??null;
  data.terminal={pass:!result.error&&(data.eventTypes??[]).includes('chat.done')&&!(data.eventTypes??[]).includes('chat.error')&&[undefined,'stop','end_turn','completed'].includes(result.finishReason)};
  data.budget={pass:!result.error&&!data.budgetStopped&&result.latencyMs<=900000&&(data.steps??[]).length<=24&&(data.toolCalls??[]).length<=32};
  data.oracle=await createArtifactOracle(task.definitions)({taskId:task.taskId,workRoot:data.workRoot,response:result.output});
  data.artifactTiming='snapshot after stream; background writers unverified';data.cleanup='retained for evidence';
  data.actualModel=environment.model;data.gatewayBuildId=health.build_id;
  data.originalProfileGrade=evaluate(result.output,{providerResponse:{...result,metadata:{...data,evaluationProfile:'runtime-accuracy'}}});
  await writeFile(path.join(directory,'result.json'),JSON.stringify(result,null,2));
  return result;
 }
}
async function run(){
 const args=process.argv.slice(2);
 if(JSON.stringify(args)!==JSON.stringify(['before','--suite','runtime-accuracy','--metadata','split=dev','--output','evals/promptfoo/results/accuracy-baseline/before.json']))throw Error('expected exact dev-only before command');
 verify();const tasks=await loadTasks(),environment=JSON.parse(await readFile(path.join(results,'live-environment.json')));
 const manifest=JSON.parse(await readFile(path.join(root,'accuracy-manifest.json')));
 manifest.stage='frozen';Object.assign(manifest.contract.experiment,{model:environment.model,endpoint:environment.endpoint,
  parameters:environment.parameters,environmentDigest:sha(environment),contextDigest:sha(tasks.map(t=>t.vars)),permissionsDigest:sha({mode:'unisolated-user-requested',configuration:environment.permissionsDigest,isolationVerified:false,backgroundSettlement:'unverified'})});
 manifest.run={sourceCommit:environment.workingTreeCommit,time:new Date().toISOString()};validateManifest(manifest);
 await writeFile(path.join(results,'before-manifest.json'),JSON.stringify(manifest,null,2),{flag:'wx'});
 await writeFile(path.join(results,'campaign.started'),JSON.stringify({manifestDigest:manifestDigest(manifest)}),{flag:'wx'});
 const config={prompts:['{{task}}'],providers:[{id:'file://'+fileURLToPath(import.meta.url)}],
  tests:tasks.map(t=>({description:t.description,metadata:t.metadata,vars:t.vars,
   assert:[{type:'javascript',value:'file://'+fileURLToPath(import.meta.url)+':grade'},
    ...(t.taskId==='finish-02'?[{type:'javascript',value:'file://'+path.join(root,'assertions.mjs')+':readOnlyTrajectory'}]:[])]})),
  defaultTest:{options:{disableVarExpansion:true}},sharing:false};
 const configPath=path.join(results,'baseline-config.json');await writeFile(configPath,JSON.stringify(config));
 const command=['--yes','promptfoo@0.122.0','eval','-c',configPath,'--repeat','3','--no-cache','-j','1',
  '--filter-metadata','suite=runtime-accuracy','--filter-metadata','split=dev','-o',path.join(results,'before.json')];
 await writeFile(path.join(results,'eval-command.json'),JSON.stringify(['npx',...command]));
 const child=spawn('npx',command,{stdio:'inherit'});const[code]=await once(child,'exit');
 await writeFile(path.join(results,'eval-exit.json'),JSON.stringify({exitCode:code}));process.exitCode=code??1;
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url))await run();

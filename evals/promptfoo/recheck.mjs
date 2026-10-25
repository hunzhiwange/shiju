// Existing Gateway, explicitly unisolated; independent task/result oracles remain unchanged.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { verify } from './freeze-accuracy.mjs';
import { validateManifest } from './accuracy-manifest.mjs';
import { createArtifactOracle } from './artifact-oracle.mjs';
import { evaluate } from './assertions.mjs';
import Provider, { aggregateEvents } from './vibewindow-provider.mjs';
const root=path.dirname(fileURLToPath(import.meta.url)), results=path.join(root,'results/accuracy-baseline');
export function campaignPaths(name='before'){
 if(!['before','recheck'].includes(name))throw Error('unsupported campaign');
 return {name,attempts:path.join(results,name==='before'?'attempts':'recheck-attempts'),output:path.join(results,`${name}.json`)};
}
export { loadTasks, grade } from './baseline.mjs';
import { loadTasks, grade } from './baseline.mjs';
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
  const campaign=campaignPaths(process.env.VIBEWINDOW_BASELINE_CAMPAIGN);
  await mkdir(campaign.attempts,{recursive:true});
  let directory;
  for(let i=1;i<=3;i++){
   const candidate=path.join(campaign.attempts,`${task.taskId}-${i}`);
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
 const campaign=campaignPaths(args.at(-1)?.endsWith('/recheck.json')?'recheck':'before');
 if(JSON.stringify(args)!==JSON.stringify(['before','--suite','runtime-accuracy','--metadata','split=dev','--output',`evals/promptfoo/results/accuracy-baseline/${campaign.name}.json`]))throw Error('expected exact dev-only before command');
 verify();const tasks=await loadTasks(),environment=JSON.parse(await readFile(path.join(results,'live-environment.json')));
 if(campaign.name!=='recheck')throw Error('recheck output required');
 validateManifest(JSON.parse(await readFile(path.join(results,'before-manifest.json'))));
 await writeFile(path.join(results,'recheck.started'),JSON.stringify({time:new Date().toISOString()}),{flag:'wx'});
 const config={prompts:['{{task}}'],providers:[{id:'file://'+fileURLToPath(import.meta.url)}],
  tests:tasks.map(t=>({description:t.description,metadata:t.metadata,vars:t.vars,
   assert:[{type:'javascript',value:'file://'+fileURLToPath(import.meta.url)+':grade'},
    ...(t.taskId==='finish-02'?[{type:'javascript',value:'file://'+path.join(root,'assertions.mjs')+':readOnlyTrajectory'}]:[])]})),
  defaultTest:{options:{disableVarExpansion:true}},sharing:false};
 const configPath=path.join(results,`${campaign.name}-config.json`);await writeFile(configPath,JSON.stringify(config));
 const command=['--yes','promptfoo@0.122.0','eval','-c',configPath,'--repeat','3','--no-cache','-j','1',
  '--filter-metadata','suite=runtime-accuracy','--filter-metadata','split=dev','-o',campaign.output];
 await writeFile(path.join(results,`${campaign.name}-eval-command.json`),JSON.stringify(['npx',...command]));
 const child=spawn('npx',command,{stdio:'inherit',env:{...process.env,VIBEWINDOW_BASELINE_CAMPAIGN:campaign.name}});const[code]=await once(child,'exit');
 await writeFile(path.join(results,`${campaign.name}-eval-exit.json`),JSON.stringify({exitCode:code}));process.exitCode=code??1;
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url))await run();

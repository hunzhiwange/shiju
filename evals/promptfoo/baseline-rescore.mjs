// Offline recovery of Promptfoo's named-export loader failure. Never calls a provider.
import { createHash } from 'node:crypto';
import { constants } from 'node:fs';
import { readFile, writeFile, rename, copyFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { grade, loadTasks } from './baseline.mjs';
import { readOnlyTrajectory } from './assertions.mjs';
import { verify } from './freeze-accuracy.mjs';
import { manifestDigest } from './accuracy-manifest.mjs';
const root=dirname(fileURLToPath(import.meta.url)), results=join(root,'results/accuracy-baseline');
verify();
const rescoreScriptSha256=createHash('sha256').update(await readFile(fileURLToPath(import.meta.url))).digest('hex');
const gradeSourceFileSha256=createHash('sha256').update(await readFile(join(root,'baseline.mjs'))).digest('hex');
const manifest=JSON.parse(await readFile(join(results,'before-manifest.json'))), digest=manifestDigest(manifest);
const sourceIdentity=JSON.parse(await readFile(join(results,'source-identity.json')));
const tasks=await loadTasks(), saved=new Map();
for(const task of tasks){
 const values=[];
 for(let index=1;index<=3;index++){
  const record=JSON.parse(await readFile(join(results,'attempts',`${task.taskId}-${index}`,'result.json')));
  if(record.metadata.taskId!==task.taskId||record.metadata.manifestDigest!==digest)throw Error('attempt identity or contract mismatch');
  if(record.metadata.fixtureDigest!==manifest.contract.tasks.find(t=>t.taskId===task.taskId).fixtureDigest)throw Error('fixture mismatch');
  values.push(record);
 }
 saved.set(task.taskId,values);
}
const records=[...saved.values()].flat();
for(const field of ['attemptId','workRoot'])if(new Set(records.map(r=>r.metadata[field])).size!==45)throw Error('attempts not independent');
const sessions=records.map(r=>r.sessionId).filter(id=>typeof id==='string'&&id);
if(new Set(sessions).size!==sessions.length)throw Error('duplicate observed session identities');
const source=join(results,'before.json'), raw=JSON.parse(await readFile(source));
const rows=raw.results.results;
if(rows.length!==45)throw Error('expected 45 raw Promptfoo rows');
const used=new Map();let recovered=0,successes=0,failures=0,errors=0,terminalCorrections=0;
for(const row of rows){
 const id=row.testCase?.metadata?.taskId??row.vars?.taskId, index=used.get(id)??0;
 const task=tasks.find(t=>t.taskId===id), record=saved.get(id)?.[index];
 if(!task||!record)throw Error('unexpected raw row identity');
 used.set(id,index+1);
 const turns=record.metadata.turns??[record];
 const terminal=!record.error&&turns.every(turn=>turn.metadata.eventTypes?.includes('chat.done')
  &&!turn.metadata.eventTypes?.includes('chat.error')&&[undefined,'stop','end_turn','completed'].includes(turn.finishReason));
 if(record.metadata.terminal?.pass!==terminal){
  record.metadata.originalSamplingTerminal=record.metadata.terminal;
  record.metadata.terminal={pass:terminal};terminalCorrections++;
 }
 const context={providerResponse:record,vars:task.vars,test:row.testCase};
 const verdict=grade(record.output,context);
 if(id==='finish-02'){
  const readOnly=readOnlyTrajectory(record.output,context);
  verdict.componentResults.push(readOnly);verdict.pass&&=readOnly.pass;verdict.score=verdict.pass?1:0;
 }
 const infrastructureError=Boolean(record.error)&&record.metadata.budgetStopped!==true;
 row.metadata={...record.metadata,sourceIdentityScope:sourceIdentity.scope,sourceIdentityDigestKind:sourceIdentity.digestKind,originalSamplingRun:record.metadata.manifest.run};
 row.metadata.manifest=structuredClone(record.metadata.manifest);
 row.metadata.manifest.run.sourceDiffDigest=sourceIdentity.sourceDiffDigest;
 const originalError=row.error??null;
 if(originalError){recovered++;row.metadata={...row.metadata,originalPromptfooGradingError:originalError};}
 row.response=record;row.tokenUsage=record.tokenUsage??row.tokenUsage;
 row.metadata={...row.metadata,usageStatus:record.tokenUsage?.total>0?'reported':'zero-or-unreported'};row.gradingResult=verdict;row.success=verdict.pass&&!record.error;
 row.metadata.failureClassification=record.error?(record.metadata.budgetStopped?'capability-budget':'runtime-or-infrastructure'):(row.success?'none':'capability');
 row.score=row.success?1:0;row.error=infrastructureError?record.error:undefined;row.namedScores=Object.assign({},...verdict.componentResults.map(c=>c.namedScores??{}));
 row.failureReason=row.success?0:infrastructureError?2:1;
 if(infrastructureError)errors++;else if(row.success)successes++;else failures++;
}
if(tasks.some(t=>used.get(t.taskId)!==3))throw Error('task repetition mismatch');
Object.assign(raw.results.stats,{successes,failures,errors});
for(const prompt of raw.results.prompts){
 const selected=rows.filter(row=>row.promptId===prompt.id);
 const relevant=selected.length?selected:rows;
 const components=relevant.flatMap(row=>row.gradingResult.componentResults);
 Object.assign(prompt.metrics,{score:relevant.reduce((n,row)=>n+row.score,0),
  testPassCount:relevant.filter(row=>row.success).length,
  testFailCount:relevant.filter(row=>!row.success&&!row.error).length,
  testErrorCount:relevant.filter(row=>row.error).length,
  assertPassCount:components.filter(c=>c.pass).length,assertFailCount:components.filter(c=>!c.pass).length});
}
raw.metadata={...raw.metadata,offlineRescore:{reason:'Promptfoo named-export loader TypeError; same frozen grade function called directly',
 originalExport:'before.promptfoo-raw.json',frozenEvaluatorDigest:manifest.contract.experiment.evaluatorDigest,
 modelAttemptsAdded:0,networkRequests:0,gradingErrorsRecovered:recovered,rescoreScriptSha256,gradeSourceFileSha256,terminalFlagsCheckedAgainstAllTurns:true,terminalCorrections,observedSessionIds:sessions.length,sessionIdsMissing:45-sessions.length,mode:'unisolated-user-requested',
 isolationVerified:false,backgroundSettlement:'unverified',sharedPersistentState:'unverified-not-isolated'}};
await copyFile(source,join(results,'before.promptfoo-raw.json'),constants.COPYFILE_EXCL);
const temporary=join(results,'before.rescored.tmp');
await writeFile(temporary,JSON.stringify(raw,null,2),{flag:'wx'});
await rename(temporary,source);
console.log(JSON.stringify({attempts:45,tasks:15,successes,failures,errors,gradingErrorsRecovered:recovered,manifestDigest:digest}));

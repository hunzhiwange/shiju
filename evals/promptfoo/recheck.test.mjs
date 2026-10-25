import test from 'node:test';
import assert from 'node:assert/strict';
test('baseline_campaign_paths_keep_recheck_evidence_separate', async()=>{
 const { campaignPaths }=await import('./recheck.mjs');
 assert.match(campaignPaths('before').attempts,/accuracy-baseline\/attempts$/);
 assert.match(campaignPaths('recheck').attempts,/accuracy-baseline\/recheck-attempts$/);
 assert.throws(()=>campaignPaths('after'),/unsupported/);
});

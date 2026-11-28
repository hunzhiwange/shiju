DELETE FROM workflow_workflows
WHERE uuid = '16cbb033-b312-48b6-9bfc-91c87391b167'
  AND workflow_yaml LIKE '%shiju!_video%' ESCAPE '!';

DROP TABLE IF EXISTS workflow_video_project_writes;
DROP TABLE IF EXISTS workflow_video_edit_runs;

INSERT INTO workflow_schema_migrations (version) VALUES (1);

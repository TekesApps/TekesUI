#!/usr/bin/env python3
"""Seal candidate source bytes. This does not approve a design or supply upstream provenance."""
from __future__ import annotations
import hashlib, json, pathlib
ROOT=pathlib.Path(__file__).resolve().parents[1]
files={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()
       for p in sorted(ROOT.rglob('*')) if p.is_file()
       and str(p.relative_to(ROOT))!='manifest.json'
       and not str(p.relative_to(ROOT)).startswith('reports/')
       and '__pycache__' not in p.parts}
aggregate=hashlib.sha256('\n'.join(f'{p}\0{h}' for p,h in sorted(files.items())).encode()).hexdigest()
m={'manifest_version':'1.0','design_version':'0.3.0-proposal.1','status':'unapproved-proposal',
   'source_artifact':'TekesUI Design System','source_revision':None,'source_export_sha256':None,
   'observed_repository_head':'a3bdf046891db2b216ef3640a2ca7f56c1893514',
   'reference_repository':'Caho1/open-claude-ui','reference_commit':'2834bc158a4849f21c97cbb2e0b10525f6e646f1',
   'reference_lock_sha256':files['research/open-claude-ui.lock.json'],
   'observed_repository_head_status':'inherited v0.1 TekesUI baseline; not reverified in this revision',
   'upstream_freshness':'unverified','source_files_atomic_repository_snapshot':False,
   'schema_id':'TekesUI documented artifact family envelope — proposed profile',
   'schema_sha256':files['design/tokens/tokens.schema.json'],
   'generator_sha256':files['tools/generate.py'],
   'context_generator_sha256':files['tools/context.py'],
   'reference_posts_sha256':files['research/claude-figma.sources.json'],
   'jenny_sources_sha256':files['research/jenny-wen.sources.json'],
   'craft_review_sha256':files['design/quality/craft-review.json'],
   'brand_compiler_version':'1.0.0',
   'brand_compiler_sha256':files['tools/brand.py'],
   'brand_color_sha256':files['tools/brand_color.py'],
   'brand_policy_sha256':files['design/brand/policy.json'],
   'brand_schema_sha256':files['design/brand/brand.schema.json'],
   'base_tokens_sha256':files['design/tokens/base.tokens.json'],
   'explicit_gallery_reference':'example-neutral',
   'hash_algorithm':'SHA-256 over original file bytes; aggregate uses sorted relative path + NUL + hex digest, joined by LF, without trailing LF',
   'bundle_source_sha256':aggregate,
   'integrity_is_not_source_authenticity':True,
   'reports_excluded_from_source_digest':True,
   'files':files}
(ROOT/'manifest.json').write_text(json.dumps(m,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Sealed proposal source bytes: '+aggregate+'; upstream approval remains unverified.')

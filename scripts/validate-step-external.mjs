// Optional independent interoperability check; no OCCT code is shipped by Hagane.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {execFileSync} from 'node:child_process';
import {readFileSync,writeFileSync,mkdtempSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
const modulePath=process.argv[2];
if(!modulePath)throw new Error('Pass an external occt-import-js installation path; see docs/step-export.md.');
const importer=await createRequire(import.meta.url)(resolve(modulePath))();
const root=new URL('../',import.meta.url);
const readDocument=file=>JSON.parse(readFileSync(new URL('../docs/'+file,import.meta.url),'utf8'));
const original=readDocument('workflow-skew-extrusion-example.json');
const reversed=structuredClone(original);reversed.operations[0].outer.reverse();for(const hole of reversed.operations[0].holes)hole.reverse();
const temporary=mkdtempSync(join(tmpdir(),'hagane-step-'));
function run(example,args){return execFileSync('cargo',['run','--quiet','--locked','--example',example,'--',...args],{cwd:root});}
function axisCrossings(mesh,xy){
 const p=mesh.attributes.position.array;const zs=[];
 for(let i=0;i<mesh.index.array.length;i+=3){
  const [a,b,c]=mesh.index.array.slice(i,i+3).map(j=>p.slice(j*3,j*3+3));
  const det=(b[0]-a[0])*(c[1]-a[1])-(c[0]-a[0])*(b[1]-a[1]);if(Math.abs(det)<1e-12)continue;
  const u=((xy[0]-a[0])*(c[1]-a[1])-(c[0]-a[0])*(xy[1]-a[1]))/det;
  const v=((b[0]-a[0])*(xy[1]-a[1])-(xy[0]-a[0])*(b[1]-a[1]))/det;
  if(u>=-1e-8&&v>=-1e-8&&u+v<=1+1e-8)zs.push(a[2]+u*(b[2]-a[2])+v*(c[2]-a[2]));
 }
 return zs.sort((a,b)=>a-b).filter((z,i,all)=>!i||Math.abs(z-all[i-1])>1e-5);
}
function check(step,{faces,volume:expectedVolume,bounds:expectedBounds,radialHeight=0,axis},scale=1){
 for(const [linearUnit,unitScale] of [['millimeter',1],['meter',0.001]]){
  const factor=scale*unitScale, deflection=0.0001*factor;
  const result=importer.ReadStepFile(step,{linearUnit,linearDeflectionType:'absolute_value',linearDeflection:deflection});
  assert.equal(result.success,true);assert.equal(result.meshes.length,1);
  const mesh=result.meshes[0];assert.equal(mesh.brep_faces.length,faces);
  const p=mesh.attributes.position.array;
  const bounds=[0,1,2].map(a=>{const coordinates=p.filter((_,i)=>i%3===a);return [Math.min(...coordinates),Math.max(...coordinates)];});
  for(let a=0;a<3;a++)for(let side=0;side<2;side++)assert.ok(Math.abs(bounds[a][side]-expectedBounds[a][side]*factor)<deflection+1e-5*factor,JSON.stringify({bounds,expectedBounds,factor}));
  let volume=0;for(let i=0;i<mesh.index.array.length;i+=3){const [a,b,c]=mesh.index.array.slice(i,i+3).map(j=>p.slice(j*3,j*3+3));volume+=(a[0]*(b[1]*c[2]-b[2]*c[1])+a[1]*(b[2]*c[0]-b[0]*c[2])+a[2]*(b[0]*c[1]-b[1]*c[0]))/6;}
  // The oracle returns triangles, not exact volume. Bound radial chord loss by
  // 2π δ Σ(radius*height); allow float32 coordinate roundoff independently.
  const budget=(2*Math.PI*0.0001*radialHeight+0.02)*factor**3;
  assert.ok(Math.abs(volume-expectedVolume*factor**3)<budget,JSON.stringify({volume,expectedVolume,factor,budget}));
  if(axis){const crossings=axisCrossings(mesh,axis.xy.map(v=>v*factor));assert.equal(crossings.length,axis.z.length);crossings.forEach((z,i)=>assert.ok(Math.abs(z-axis.z[i]*factor)<1e-5*factor));}
 }
}
try{
 const bounds=[[-40,58],[-42,30],[-12,12]];
 const cases=[
  {document:original,faces:14,volume:105792,bounds},
  {document:reversed,faces:14,volume:105792,bounds},
  {document:readDocument('workflow-skew-bores-example.json'),faces:15,volume:105792-Math.PI*16*24,bounds,radialHeight:4*24,axis:{xy:[24,0],z:[]}},
  {document:readDocument('workflow-bottom-blind-example.json'),faces:16,volume:105792-Math.PI*16*8,bounds,radialHeight:4*8,axis:{xy:[24,0],z:[-4,12]}},
  {document:readDocument('workflow-opposing-blind-example.json'),faces:18,volume:105792-Math.PI*(16*8+36*10),bounds,radialHeight:4*8+6*10,axis:{xy:[24,0],z:[-4,2]}},
 ];
 for(const fixture of cases){const input=join(temporary,'workflow.json');writeFileSync(input,JSON.stringify(fixture.document));check(run('step_export',[input]),fixture);}
 const unit=[1/Math.sqrt(14),2/Math.sqrt(14),3/Math.sqrt(14)],cos=Math.cos(0.7),sin=Math.sin(0.7);
 const axis=[unit[1]*sin+unit[0]*unit[2]*(1-cos),-unit[0]*sin+unit[1]*unit[2]*(1-cos),cos+unit[2]**2*(1-cos)];
 const center=[20,-7,12];const placedBounds=axis.map((n,i)=>{const extent=8*Math.sqrt(1-n*n)+12*Math.abs(n);return [center[i]-extent,center[i]+extent];});
 for(const scale of [1,1000])for(const mode of ['cylinder','tube','placed-tube']){
  const tube=mode!=='cylinder';check(run('step_analytic',[mode,String(scale)]),{faces:tube?4:3,volume:Math.PI*(tube?48:64)*24,bounds:mode==='placed-tube'?placedBounds:[[-8,8],[-8,8],[-12,12]],radialHeight:(tube?12:8)*24},scale);
 }
 console.log('External STEP import: planes, circles, cylinders, tubes, rigid placement, through/blind/opposing holes, face counts, bounds, bounded mesh volume, retained floors and mm/metre conversion passed.');
}finally{rmSync(temporary,{recursive:true,force:true});}

// Optional OCCT oracle (occt-import-js 0.0.23, LGPL); never a kernel dependency.
// Imported triangle volumes are approximate, compared with independent polynomial integration.
// POLYGON_STEP_DEFLECTION selects requested reader deflection in mm (default .0001).
// POLYGON_STEP_CASE selects a zero-based fixture for diagnosis. Reader deflection
// is a request, not a certificate: geometry bounds use actual recovered UV triangles.
// For barycentric weights, Taylor error <= (H/2)*sum_{i<j} lambda_i*lambda_j*d_ij²
// <= H*maxEdgeUV²/6. H is a Hessian row-sum bound on the float32-expanded domain.
// Float32 position error contributes a separate roof Lipschitz allowance.
// The .0001 mm default-pentagon meter read measurably exceeds its requested
// deflection+original coordinate guard; the script reports this reader limitation.
// Exact STEP roof control points are independently reconstructed before meshing.
// Floating arithmetic allowances are engineering guards, not interval certificates.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {execFileSync} from 'node:child_process';
import {resolve} from 'node:path';
const installation=process.argv[2];
if(!installation)throw new Error('Pass an external occt-import-js installation path.');
const reader=await createRequire(import.meta.url)(resolve(installation))();
const pent=[[.15,.25],[.65,.1],[.9,.45],[.65,.85],[.2,.8]],diamond=[[.3,.5],[.5,.3],[.7,.5],[.5,.7]];
const regular=(n,r)=>Array.from({length:n},(_,i)=>[.5+r*Math.cos(2*Math.PI*i/n),.5+r*Math.sin(2*Math.PI*i/n)]);
const model=[80,60,20,30,.5,0,0,0,0,0,1,0,1];
const cases=[{name:'plain pentagon',model,outer:pent,hole:[]},{name:'pentagon diamond opening',model,outer:pent,hole:diamond},{name:'trimmed signed placed opening',model:[20,12,3,-2,.05,-.7,-120,30,90,.15,.85,.1,.9],outer:[[.2,.2],[.7,.15],[.8,.55],[.55,.8],[.25,.7]],hole:[[.35,.45],[.48,.3],[.65,.5],[.45,.65]]},{name:'three corners',model,outer:[[.1,.1],[.9,.2],[.3,.9]],hole:[]},{name:'sixteen corner rings',model,outer:regular(16,.44),hole:regular(16,.16)}];
const factorial=n=>n<2?1:n*factorial(n-1);
function multiply(a,b){const c=new Map();for(const [k,x] of a)for(const [l,y] of b){const [i,j]=k.split(',').map(Number),[p,q]=l.split(',').map(Number),key=`${i+p},${j+q}`;c.set(key,(c.get(key)??0)+x*y);}return c;}
function moment(poly,p,q){let integral=0;for(let k=1;k+1<poly.length;k++){const [a,b,c]=[poly[0],poly[k],poly[k+1]],det=(b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0]),u=new Map([['0,0',a[0]],['1,0',b[0]-a[0]],['0,1',c[0]-a[0]]]),v=new Map([['0,0',a[1]],['1,0',b[1]-a[1]],['0,1',c[1]-a[1]]]);let power=new Map([['0,0',1]]);for(let i=0;i<p;i++)power=multiply(power,u);for(let j=0;j<q;j++)power=multiply(power,v);for(const [key,value]of power){const [i,j]=key.split(',').map(Number);integral+=det*value*factorial(i)*factorial(j)/factorial(i+j+2);}}return integral;}
function crossings(points,index,x,y,guard){const hits=[];for(let i=0;i<index.length;i+=3){const [a,b,c]=index.slice(i,i+3).map(id=>points[id]),det=(b[0]-a[0])*(c[1]-a[1])-(c[0]-a[0])*(b[1]-a[1]);if(Math.abs(det)<1e-12)continue;const u=((x-a[0])*(c[1]-a[1])-(y-a[1])*(c[0]-a[0]))/det,v=((b[0]-a[0])*(y-a[1])-(b[1]-a[1])*(x-a[0]))/det;if(u>=-1e-8&&v>=-1e-8&&u+v<=1+1e-8)hits.push(a[2]+u*(b[2]-a[2])+v*(c[2]-a[2]));}return hits.sort((a,b)=>a-b).filter((z,i,all)=>i===0||z-all[i-1]>guard);}
const requestedDeflection=Number(process.env.POLYGON_STEP_DEFLECTION??1e-4);
assert.ok(Number.isFinite(requestedDeflection)&&requestedDeflection>0,'reader deflection must be finite and positive');
const selected=cases.filter((_,i)=>!process.env.POLYGON_STEP_CASE || i===Number(process.env.POLYGON_STEP_CASE));
assert.ok(selected.length>0,'requested fixture does not exist');
for(const test of selected){const {outer,hole}=test,[L,W,H,bulge,,angle,tx,ty,tz]=test.model,mode=Number(hole.length>0),values=[...test.model,...(mode?[outer.length,hole.length]:[]),...outer.flat(),...hole.flat()],report=JSON.parse(execFileSync('cargo',['run','--quiet','--locked','--example','nurbs_graph_polygon_step','--',String(mode),JSON.stringify(values)],{encoding:'utf8',maxBuffer:32*1024*1024,cwd:new URL('../',import.meta.url)})),step=report.step,faces=2+outer.length+hole.length;
 assert.equal((step.match(/ADVANCED_FACE\(/g)??[]).length,faces);assert.equal((step.match(/B_SPLINE_SURFACE_WITH_KNOTS\(/g)??[]).length,faces);assert.ok(step.includes('MANIFOLD_SOLID_BREP'));assert.ok(!step.includes('TRIANGULATED_FACE_SET'));
 // Independently reconstruct the encoded unit-weight quadratic roof basis.
 // Decimal STEP control points round-trip as doubles; U-major rows must retain
 // the original source UV domain and placement, without any fitted geometry.
 const roofLine=step.split('\n').filter(line=>line.includes("=B_SPLINE_SURFACE_WITH_KNOTS('',2,2,"))[1];
 assert.ok(roofLine,'quadratic cap basis missing');
 const controlIds=[...roofLine.split('.UNSPECIFIED.')[0].matchAll(/#(\d+)/g)].slice(1).map(match=>Number(match[1]));
 assert.equal(controlIds.length,9);
 const entities=new Map(step.split('\n').map(line=>{const match=line.match(/^#(\d+)=(.*);$/);return match?[Number(match[1]),match[2]]:[-1,''];}));
 const controls=controlIds.map(id=>{const match=entities.get(id).match(/^CARTESIAN_POINT\('',\((.*)\)\)$/);assert.ok(match);return match[1].split(',').map(Number);});
 const localPoint=p=>{const x=p[0]-tx,y=p[1]-ty,z=p[2]-tz;return [Math.cos(angle)*x-Math.sin(angle)*z,y,Math.sin(angle)*x+Math.cos(angle)*z];};
 const basis=t=>[(1-t)**2,2*t*(1-t),t*t];
 for(let i=0;i<=8;i++)for(let j=0;j<=8;j++){const bu=basis(i/8),bv=basis(j/8),encoded=[0,1,2].map(axis=>controls.reduce((sum,p,k)=>sum+bu[Math.floor(k/3)]*bv[k%3]*p[axis],0)),local=localPoint(encoded),u=test.model[9]+(test.model[10]-test.model[9])*i/8,v=test.model[11]+(test.model[12]-test.model[11])*j/8,exact=[L*u,W*v,H+4*bulge*u*(1-u)*v*(1-v)],allowance=4096*Number.EPSILON*Math.max(L+W+H+Math.abs(bulge),Math.abs(tx)+Math.abs(ty)+Math.abs(tz));for(let axis=0;axis<3;axis++)assert.ok(Math.abs(local[axis]-exact[axis])<=allowance,'encoded roof basis differs from source polynomial');}
 const measure=(p,q)=>moment(outer,p,q)-(hole.length?moment(hole,p,q):0),area=L*W*measure(0,0),expected=L*W*(H*measure(0,0)+4*bulge*(measure(1,1)-measure(2,1)-measure(1,2)+measure(2,2))),height=(u,v)=>H+4*bulge*u*(1-u)*v*(1-v),slope=Math.sqrt(1+(bulge/L)**2+(bulge/W)**2),perimeter=p=>p.reduce((sum,a,i)=>{const b=p[(i+1)%p.length];return sum+Math.hypot(L*(a[0]-b[0]),W*(a[1]-b[1]));},0),surfaceArea=area*(1+slope)+(perimeter(outer)+(hole.length?perimeter(hole):0))*(H+Math.max(bulge,0)/4),worldScale=Math.max(Math.abs(tx),Math.abs(ty),Math.abs(tz),L+W+H+Math.abs(bulge)),roundoff=4*2**-23*worldScale*Math.sqrt(3),deflection=requestedDeflection,coordinateGuard=roundoff*(1+Math.abs(bulge)/L+Math.abs(bulge)/W)+1e-8;
 for(const [linearUnit,scale]of [['millimeter',1],['meter',.001]]){
  const result=reader.ReadStepFile(Buffer.from(step),{linearUnit,linearDeflectionType:'absolute_value',linearDeflection:deflection*scale});assert.equal(result.success,true,test.name);assert.equal(result.meshes.length,1);const mesh=result.meshes[0];assert.equal(mesh.brep_faces.length,faces);const array=mesh.attributes.position.array,points=Array.from({length:array.length/3},(_,i)=>array.slice(i*3,i*3+3)),index=mesh.index.array;
  // Exact float32 coordinate equality only: no tolerance welding to manufacture closure.
  const nodes=new Map(),ids=points.map(p=>{const key=p.join(',');if(!nodes.has(key))nodes.set(key,nodes.size);return nodes.get(key);}),edges=new Map(),reference=points[index[0]];let volume=0;
  for(let i=0;i<index.length;i+=3){const tri=index.slice(i,i+3),[a,b,c]=tri.map(id=>points[id].map((x,k)=>x-reference[k]));volume+=(a[0]*(b[1]*c[2]-b[2]*c[1])+a[1]*(b[2]*c[0]-b[0]*c[2])+a[2]*(b[0]*c[1]-b[1]*c[0]))/6;for(let k=0;k<3;k++){const a=ids[tri[k]],b=ids[tri[(k+1)%3]],key=`${Math.min(a,b)},${Math.max(a,b)}`,entry=edges.get(key)??[0,0];entry[0]++;entry[1]+=a<b?1:-1;edges.set(key,entry);}}
  for(const entry of edges.values())assert.deepEqual(entry,[2,0],`${test.name}: open or unoriented imported mesh`);
  assert.ok(volume>0);
  const cosine=Math.cos(angle),sine=Math.sin(angle),local=points.map(p=>{const [x,y,z]=p.map((v,i)=>v/scale-[tx,ty,tz][i]);return [cosine*x-sine*z,y,sine*x+cosine*z];});
  // Use the reader's actual face ranges, in serialized B-rep face order.
  // Every triangle must belong to exactly one face and pass that face's
  // geometry checks; no vertex-membership filter may silently skip it.
  const triangleFaces=new Int32Array(index.length/3).fill(-1),covered=new Uint8Array(index.length/3);
  mesh.brep_faces.forEach((face,fi)=>{assert.ok(Number.isInteger(face.first)&&Number.isInteger(face.last)&&face.first>=0&&face.last>=face.first&&face.last<triangleFaces.length);for(let ti=face.first;ti<=face.last;ti++){assert.equal(triangleFaces[ti],-1,'overlapping imported face ranges');triangleFaces[ti]=fi;}});
  assert.ok(triangleFaces.every(fi=>fi>=0),'unassigned imported triangle');
  for(let ti=0;ti<triangleFaces.length;ti++)if(triangleFaces[ti]===0){const triangle=index.slice(ti*3,ti*3+3).map(id=>local[id]);assert.ok(triangle.every(p=>p.every(Number.isFinite)&&Math.abs(p[2])<=coordinateGuard),JSON.stringify({case:test.name,linearUnit,face:0,triangle:ti,vertices:triangle,coordinateGuard}));covered[ti]=1;}
  // Imported roof triangles: corresponding XY samples test the retained U-major
  // polynomial, including rational STEP restrictions, against actual UV-span bounds.
  let roofSamples=0,maxRoofError=0,maxSample,maxRoofBound=0;
  const uvExtension=roundoff/Math.min(L,W),hessianRowBound=8*Math.abs(bulge)*Math.max(.25,uvExtension*(1+uvExtension))+4*Math.abs(bulge)*(1+2*uvExtension)**2;
  for(let i=0;i<index.length;i+=3){const triangle=index.slice(i,i+3).map(id=>local[id]);if(triangleFaces[i/3]!==1)continue;assert.ok(triangle.every(p=>p.every(Number.isFinite)&&Math.abs(p[2]-height(p[0]/L,p[1]/W))<=coordinateGuard),JSON.stringify({case:test.name,linearUnit,face:1,triangle:i/3,vertices:triangle,coordinateGuard}));covered[i/3]=1;const maxEdgeUVSquared=Math.max(...triangle.map((p,k)=>{const q=triangle[(k+1)%3];return ((p[0]-q[0])/L)**2+((p[1]-q[1])/W)**2;})),triangleBound=hessianRowBound*maxEdgeUVSquared/6+coordinateGuard;maxRoofBound=Math.max(maxRoofBound,triangleBound);for(let a=0;a<=4;a++)for(let b=0;b<=4-a;b++){const weights=[a/4,b/4,(4-a-b)/4],p=[0,1,2].map(axis=>triangle.reduce((sum,q,k)=>sum+weights[k]*q[axis],0));const deviation=Math.abs(p[2]-height(p[0]/L,p[1]/W));if(deviation>maxRoofError){maxRoofError=deviation;maxSample={triangle:i/3,vertices:triangle,weights,point:p,height:height(p[0]/L,p[1]/W),triangleBound};}assert.ok(deviation<=triangleBound,JSON.stringify({case:test.name,linearUnit,deviation,triangleBound}));roofSamples++;}}
  // Ruled walls occupy their exact vertical edge planes. Dense triangle
  // samples must remain within the source edge segment and true roof height.
  let wallSamples=0;
  const boundaries=[outer,...(hole.length?[hole]:[])].flatMap(poly=>poly.map((a,i)=>[a,poly[(i+1)%poly.length]]));
  for(let i=0;i<index.length;i+=3){const fi=triangleFaces[i/3];if(fi<2)continue;const triangle=index.slice(i,i+3).map(id=>local[id]),[a,b]=boundaries[fi-2],start=[L*a[0],W*a[1]],delta=[L*(b[0]-a[0]),W*(b[1]-a[1])],length=Math.hypot(...delta),distance=p=>Math.abs(delta[0]*(p[1]-start[1])-delta[1]*(p[0]-start[0]))/length,parameter=p=>((p[0]-start[0])*delta[0]+(p[1]-start[1])*delta[1])/length**2;
   assert.ok(triangle.every(p=>p.every(Number.isFinite)&&distance(p)<=coordinateGuard&&parameter(p)>=-coordinateGuard/length&&parameter(p)<=1+coordinateGuard/length&&p[2]>=-coordinateGuard&&p[2]<=height(p[0]/L,p[1]/W)+coordinateGuard),JSON.stringify({case:test.name,linearUnit,face:fi,triangle:i/3,vertices:triangle,edge:[a,b],coordinateGuard}));covered[i/3]=1;
   for(let r=0;r<=4;r++)for(let q=0;q<=4-r;q++){const weights=[r/4,q/4,(4-r-q)/4],p=[0,1,2].map(axis=>triangle.reduce((sum,x,k)=>sum+weights[k]*x[axis],0)),t=parameter(p);assert.ok(distance(p)<=coordinateGuard);assert.ok(t>=-coordinateGuard/length&&t<=1+coordinateGuard/length);assert.ok(p[2]>=-coordinateGuard&&p[2]<=height(p[0]/L,p[1]/W)+maxRoofBound);wallSamples++;}}
  assert.ok(covered.every(value=>value===1),'imported triangle escaped geometry validation');

  assert.ok(wallSamples>0);
  assert.ok(roofSamples>0);const budget=(maxRoofBound*area+roundoff*surfaceArea)*scale**3;assert.ok(Math.abs(volume-expected*scale**3)<=budget,JSON.stringify({case:test.name,linearUnit,volume,expected:expected*scale**3,budget}));
  if(maxRoofError>deflection*slope+roundoff+1e-8)console.log(JSON.stringify({reader_deflection_exceeded:true,case:test.name,linearUnit,requested_mm:deflection,maxRoofError,requestedDeflectionGuard:deflection*slope+roundoff+1e-8,maxSample,faceRanges:mesh.brep_faces.map(f=>({first:f.first,last:f.last})),independentTriangleBound:maxRoofBound}));
  const stock=[(outer[0][0]+outer[1][0])/2,(outer[0][1]+outer[1][1])/2],center=outer.reduce((p,q)=>p.map((v,k)=>v+q[k]/outer.length),[0,0]),query=stock.map((v,k)=>.85*v+.15*center[k]),hits=crossings(local,index,L*query[0],W*query[1],coordinateGuard);assert.equal(hits.length,2,`${test.name}: material ray`);assert.ok(Math.abs(hits[0])<=coordinateGuard);assert.ok(Math.abs(hits[1]-height(...query))<=maxRoofBound);
  if(hole.length){const center=hole.reduce((p,q)=>p.map((v,k)=>v+q[k]/hole.length),[0,0]);assert.deepEqual(crossings(local,index,L*center[0],W*center[1],coordinateGuard),[],`${test.name}: void ray`);}
  console.log(`${test.name}: ${linearUnit}, ${faces} faces, ${index.length/3} triangles, all triangles covered, exact-coordinate closure, approximate volume and ${roofSamples} roof / ${wallSamples} wall samples passed.`);
 }
}
console.log(`Independent polygon STEP oracle passed ${selected.length} documents in mm/metre. Imported mesh volumes are approximate; analytic expected volumes use independent polynomial simplex integration.`);

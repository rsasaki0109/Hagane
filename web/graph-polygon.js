import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try{
 const viewer=createMeshViewer($('view')),response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');
 const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;window.haganeGraphPolygon={ready:false,data:null};
 let acceptedValues=null,displayCenter=[0,0,0],boundarySegments=[],pointSegments=[];
 function renderOverlays(){viewer.setSegments([...boundarySegments,...pointSegments].map(segment=>segment.map(point=>point.map((x,i)=>x-displayCenter[i]))));}
 function generate(){
  const polygon=JSON.parse($('polygon').value);if(!Array.isArray(polygon)||polygon.length<3||polygon.length>16||polygon.some(p=>!Array.isArray(p)||p.length!==2||p.some(x=>typeof x!=='number'||!Number.isFinite(x))))throw Error('Polygon requires 3–16 finite numeric UV pairs.');
  const values=['width','depth','height','bulge','surface-error'].map(id=>Number($(id).value));values.push(Number($('angle').value)*Math.PI/180,...['tx','ty','tz','u-min','u-max','v-min','v-max'].map(id=>Number($(id).value)),...polygon.flat());
  k.hagane_graph_polygon_begin();for(const value of values)if(k.hagane_graph_polygon_push(value))throw Error('Polygon graph transport rejected nonfinite or excessive input.');
  const status=k.hagane_graph_polygon_finish(),data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);
  const center=[0,1,2].map(axis=>{let lo=Infinity,hi=-Infinity;for(let i=axis;i<data.positions.length;i+=3){lo=Math.min(lo,data.positions[i]);hi=Math.max(hi,data.positions[i]);}return (lo+hi)/2;});
  const segments=[];for(const samples of data.boundary_samples)for(let i=3;i<samples.length;i+=3)segments.push([samples.slice(i-3,i),samples.slice(i,i+3)]);
  viewer.setMesh({positions:data.positions.map((x,i)=>x-center[i%3]),normals:data.normals});viewer.setSegments(segments.map(segment=>segment.map(point=>point.map((x,i)=>x-center[i]))));window.haganeGraphPolygon.data=data;acceptedValues=values.slice();window.haganeGraphPolygon.step=null;window.haganeGraphPolygon.step_error=null;$('step-status').textContent='Exports the accepted B-rep model only.';displayCenter=center;boundarySegments=segments;pointSegments=[];window.haganeGraphPolygon.query=null;window.haganeGraphPolygon.query_error=null;$('classification').textContent='Model accepted. Classify a world point.';$('classification-error').textContent='';$('source-point').textContent='—';
  $('surface-error-label').value=data.error.toFixed(3);$('volume').textContent=data.volume.toFixed(3)+' mm³';$('centroid').textContent=data.mass_properties.centroid.map(x=>x.toFixed(4)).join(', ');$('inertia').textContent=data.inertia_properties?data.inertia_properties.inertia.map(row=>row.map(x=>x.toExponential(5)).join('  ')).join('\n'):'Unavailable: '+(data.inertia_error??'No inertia result');$('faces').textContent=String(data.brep.faces);$('edges').textContent=String(data.brep.edges);$('triangles').textContent=String(data.mesh.triangles.length);$('surface-bound').textContent=data.error_bounds.reduce((maximum,bound)=>Math.max(maximum,bound),0).toExponential(3);$('status').textContent='✓ Closed convex polygon B-rep validated · exact roof/walls · bounded triangles';
 }
 function downloadStep(){
  if(!acceptedValues)return;
  try{k.hagane_graph_polygon_step_begin();for(const value of acceptedValues)if(k.hagane_graph_polygon_step_push(value))throw Error('STEP transport rejected model input.');const status=k.hagane_graph_polygon_step_finish(),result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(result.error);
   const url=URL.createObjectURL(new Blob([result.step],{type:'application/step'})),link=document.createElement('a');link.href=url;link.download='hagane-graph-polygon.step';document.body.append(link);link.click();link.remove();setTimeout(()=>URL.revokeObjectURL(url),1000);window.haganeGraphPolygon.step=result;window.haganeGraphPolygon.step_error=null;$('step-status').textContent='✓ Exact accepted B-rep exported · AP214 · mm · retained NURBS weights';
  }catch(e){window.haganeGraphPolygon.step_error=e.message;$('step-status').textContent='STEP export rejected: '+e.message+' Accepted model and point query are preserved.';}
 }
 $('download-step').addEventListener('click',downloadStep);
 function classify(){
  if(!acceptedValues)return;
  try{const point=['point-x','point-y','point-z'].map(id=>$(id).valueAsNumber),tolerance=$('point-tolerance').valueAsNumber;
   if(!point.every(Number.isFinite))throw Error('World XYZ must contain three finite numbers.');
   if(!Number.isFinite(tolerance)||tolerance<=0)throw Error('Euclidean tolerance must be a finite positive number.');
   if(!point.every((x,i)=>Math.abs(x-displayCenter[i])<1e30))throw Error('World point is outside the finite marker display range.');
   const values=[...acceptedValues,...point,tolerance];k.hagane_graph_polygon_point_begin();for(const value of values)if(k.hagane_graph_polygon_point_push(value))throw Error('Point query transport rejected input.');
   const status=k.hagane_graph_polygon_point_finish(),query=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(query.error);
   const cross=[];for(let axis=0;axis<3;axis++){const a=[...point],b=[...point];a[axis]-=2.5;b[axis]+=2.5;cross.push([a,b]);}pointSegments=cross;renderOverlays();window.haganeGraphPolygon.query=query;window.haganeGraphPolygon.query_error=null;$('classification').textContent=query.location+' · '+query.reason;$('source-point').textContent=query.source_point.map(x=>x.toFixed(5)).join(', ');$('classification-error').textContent='';
  }catch(e){window.haganeGraphPolygon.query_error=e.message;$('classification-error').textContent='Query rejected: '+e.message+' Last accepted result and marker are preserved.';}
 }
 $('classify').addEventListener('click',classify);
 const update=()=>{try{generate();}catch(e){$('status').textContent='Polygon graph rejected: '+e.message;}};
 $('apply').addEventListener('click',update);$('surface-error').addEventListener('input',update);$('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('reset').addEventListener('click',()=>viewer.reset());generate();window.haganeGraphPolygon.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}

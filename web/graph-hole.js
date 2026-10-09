import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try{
 const viewer=createMeshViewer($('view')),response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');
 const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;window.haganeGraphHole={ready:false,data:null};
 let accepted=null,displayCenter=[0,0,0],boundarySegments=[],pointSegments=[],sectionSegments=[];
 function generate(){
  const [width,depth,height,bulge,error]=['width','depth','height','bulge','surface-error'].map(id=>Number($(id).value));
  const angle=Number($('angle').value)*Math.PI/180,[tx,ty,tz]=['tx','ty','tz'].map(id=>Number($(id).value));
  const domain=['u-min','u-max','v-min','v-max'].map(id=>Number($(id).value));
  const hole=['hole-u0','hole-u1','hole-v0','hole-v1'].map(id=>Number($(id).value));
  const status=k.hagane_generate_graph_solid_hole(width,depth,height,bulge,error,angle,tx,ty,tz,...domain,...hole),data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);
  acceptModel(data);
 }
 function acceptModel(data){
  const error=data.error;
  const center=[0,1,2].map(axis=>{let lo=Infinity,hi=-Infinity;for(let i=axis;i<data.positions.length;i+=3){lo=Math.min(lo,data.positions[i]);hi=Math.max(hi,data.positions[i]);}return (lo+hi)/2;});
  const segments=[];for(const samples of data.boundary_samples)for(let i=3;i<samples.length;i+=3)segments.push([samples.slice(i-3,i),samples.slice(i,i+3)]);
  viewer.setMesh({positions:data.positions.map((x,i)=>x-center[i%3]),normals:data.normals});viewer.setSegments(segments.map(segment=>segment.map(point=>point.map((x,i)=>x-center[i]))));window.haganeGraphHole.data=data;accepted=data;displayCenter=center;boundarySegments=segments;window.haganeGraphHole.query=null;$('classification').textContent='Model accepted. Classify a world point.';window.haganeGraphHole.section=null;pointSegments=[];sectionSegments=[];$('section-result').textContent='Model accepted. Query a source-vertical section.';window.haganeGraphHole.step=null;$('step-status').textContent='Exports the accepted B-rep model.';window.haganeGraphHole.imported=data.import??null;$('import-status').textContent=data.import?'✓ Canonical holed graph imported · exact basis and topology validated · mm':'Full source UV · unplaced · one rectangular through hole.';
  $('surface-error-label').value=error.toFixed(3);$('volume').textContent=data.volume.toFixed(3)+' mm³';$('centroid').textContent=data.mass_properties.centroid.map(x=>x.toFixed(4)).join(', ');$('removed-volume').textContent=data.removed_volume.toFixed(3)+' mm³';$('source-volume').textContent=data.source_volume.toFixed(3)+' mm³';$('faces').textContent=String(data.brep.faces);$('edges').textContent=String(data.brep.edges);$('triangles').textContent=String(data.mesh.triangles.length);$('surface-bound').textContent=data.error_bounds.reduce((maximum,bound)=>Math.max(maximum,bound),0).toExponential(3);$('status').textContent='✓ Closed genus-one B-rep validated · exact inner walls · bounded triangles';
 }
 function showPoint(point){
  if(!point.every((x,i)=>Number.isFinite(x-displayCenter[i])&&Math.abs(x-displayCenter[i])<1e30))return;
  const cross=[];for(let axis=0;axis<3;axis++){const a=[...point],b=[...point];a[axis]-=2.5;b[axis]+=2.5;cross.push([a,b]);}
  pointSegments=cross;renderOverlays();
 }
 function renderOverlays(){viewer.setSegments([...boundarySegments,...pointSegments,...sectionSegments].map(segment=>segment.map(point=>point.map((x,i)=>x-displayCenter[i]))));}
 function classify(){
  if(!accepted)return;const p=['point-x','point-y','point-z'].map(id=>Number($(id).value)),tolerance=Number($('point-tolerance').value),d=accepted,domain=d.source_domain.flat(),hole=d.hole.flat();
  try{const status=k.hagane_classify_graph_hole(d.width,d.depth,d.height,d.bulge,d.error,d.placement.angle,...d.placement.translation,...domain,...hole,...p,tolerance),query=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(query.error);window.haganeGraphHole.query=query;$('classification').textContent=query.location+' · '+query.reason;if(p.every(Number.isFinite))showPoint(p);}
  catch(e){window.haganeGraphHole.query={error:e.message,point:p};$('classification').textContent='Query rejected: '+e.message;if(p.every(Number.isFinite))showPoint(p);}
 }
 $('classify').addEventListener('click',classify);
 function verticalSection(){
  if(!accepted)return;const uv=['section-u','section-v'].map(id=>Number($(id).value)),linear=Number($('section-tolerance').value),d=accepted,domain=d.source_domain.flat(),hole=d.hole.flat();
  try{const status=k.hagane_section_graph_hole(d.width,d.depth,d.height,d.bulge,d.error,d.placement.angle,...d.placement.translation,...domain,...hole,...uv,linear),section=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(section.error);window.haganeGraphHole.section=section;
   const line=section.line,at=t=>line.origin.map((x,i)=>x+t*line.direction[i]),extent=d.height+Math.abs(d.bulge)/4+5;
   sectionSegments=[[at(-5),at(extent)],...section.segments.map(segment=>[segment.a,segment.b]),...section.events.map(event=>[event.point,event.point.map((x,i)=>x+6*event.normal[i])])];renderOverlays();
   $('section-result').textContent=section.intervals.length?'Material interval '+section.intervals.map(r=>'['+r.map(x=>x.toFixed(4)).join(', ')+']').join(' · ')+' · length '+section.length.toFixed(4)+' mm · '+section.events.map(e=>(e.entering?'enter':'exit')+' face '+e.face+' normal ('+e.normal.map(x=>x.toFixed(3)).join(', ')+')').join(' · '):'Empty material section · original source-vertical line retained.';
  }catch(e){window.haganeGraphHole.section={error:e.message,source_uv:uv};$('section-result').textContent='Section rejected: '+e.message;}
 }
 $('section-query').addEventListener('click',verticalSection);


 function downloadStep(){
  if(!accepted)return;const d=accepted,domain=d.source_domain.flat(),hole=d.hole.flat();
  try{const status=k.hagane_export_graph_hole_step(d.width,d.depth,d.height,d.bulge,d.error,d.placement.angle,...d.placement.translation,...domain,...hole),result=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(result.error);window.haganeGraphHole.step=result;
   const url=URL.createObjectURL(new Blob([result.step],{type:'application/step'})),link=document.createElement('a');link.href=url;link.download='hagane-graph-through-hole.step';document.body.append(link);link.click();link.remove();setTimeout(()=>URL.revokeObjectURL(url),1000);$('step-status').textContent='✓ Exact accepted B-rep exported · AP214 · mm';
  }catch(e){$('step-status').textContent='STEP export rejected: '+e.message;}
 }
 $('download-step').addEventListener('click',downloadStep);
 function importStep(text){
  try{const bytes=new TextEncoder().encode(text);if(bytes.length>1024*1024)throw Error('STEP import exceeds 1 MiB.');k.hagane_graph_hole_step_import_begin();for(const byte of bytes)if(k.hagane_graph_hole_step_import_push_byte(byte))throw Error('Graph STEP transport rejected.');
   const status=k.hagane_graph_hole_step_import_finish(Number($('surface-error').value)),data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);acceptModel(data);
   for(const [id,value]of [['width',data.width],['depth',data.depth],['height',data.height],['bulge',data.bulge],['angle',0],['tx',0],['ty',0],['tz',0],['u-min',0],['u-max',1],['v-min',0],['v-max',1],['hole-u0',data.hole[0][0]],['hole-u1',data.hole[0][1]],['hole-v0',data.hole[1][0]],['hole-v1',data.hole[1][1]]])$(id).value=String(value);
   $('import-status').textContent='✓ Canonical holed graph imported · exact basis and topology validated · mm';window.haganeGraphHole.imported=data.import;$('step-file').value='';
  }catch(e){$('import-status').textContent='STEP import rejected: '+e.message;}
 }
 $('import-step').addEventListener('click',()=>importStep($('step-document').value));
 $('step-file').addEventListener('change',async()=>{const file=$('step-file').files[0];if(!file)return;try{if(file.size>1024*1024)throw Error('STEP import exceeds 1 MiB.');const buffer=await file.arrayBuffer();let text;try{text=new TextDecoder('utf-8',{fatal:true}).decode(buffer);}catch{throw Error('STEP input must be valid UTF-8.');}$('step-document').value=text;importStep(text);}catch(e){$('import-status').textContent='STEP import rejected: '+e.message;}});
 const update=()=>{try{generate();}catch(e){$('status').textContent=e.message;}};$('apply').addEventListener('click',update);$('surface-error').addEventListener('input',update);$('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('reset').addEventListener('click',()=>viewer.reset());generate();window.haganeGraphHole.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}

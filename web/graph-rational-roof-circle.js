import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try{
 const viewer=createMeshViewer($('view')),response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');
 const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;window.haganeGraphRationalRoofCircle={ready:false,data:null};
 function generate(){
  const values=['width','depth','height','bulge','surface-error'].map(id=>$(id).valueAsNumber);values.push($('angle').valueAsNumber*Math.PI/180,...['tx','ty','tz','u-min','u-max','v-min','v-max','circle-x','circle-y','circle-radius'].map(id=>$(id).valueAsNumber));
  k.hagane_graph_rational_roof_circle_begin();for(const value of values)if(k.hagane_graph_rational_roof_circle_push(value))throw Error('Circle path transport rejected nonfinite input.');const status=k.hagane_graph_rational_roof_circle_finish(),data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);
  const center=[0,1,2].map(axis=>{let lo=Infinity,hi=-Infinity;for(let i=axis;i<data.positions.length;i+=3){lo=Math.min(lo,data.positions[i]);hi=Math.max(hi,data.positions[i]);}return(lo+hi)/2;}),segments=[];
  for(const samples of data.boundary_samples)for(let i=3;i<samples.length;i+=3)segments.push([samples.slice(i-3,i),samples.slice(i,i+3)]);
  for(const quarter of data.rational_roof_path.quarters)for(let i=1;i<quarter.points.length;i++)segments.push([quarter.points[i-1],quarter.points[i]]);
  viewer.setMesh({positions:data.positions.map((x,i)=>x-center[i%3]),normals:data.normals});viewer.setSegments(segments.map(segment=>segment.map(point=>point.map((x,i)=>x-center[i]))));window.haganeGraphRationalRoofCircle.data=data;
  $('surface-error-label').value=data.error.toFixed(3);$('volume').textContent=data.volume.toFixed(3)+' mm³';$('centroid').textContent=data.mass_properties.centroid.map(x=>x.toFixed(4)).join(', ');$('inertia').textContent=data.inertia_properties?data.inertia_properties.inertia.map(row=>row.map(x=>x.toExponential(5)).join('  ')).join('\n'):'Unavailable: '+data.inertia_error;$('faces').textContent=String(data.brep.faces);$('edges').textContent=String(data.brep.edges);$('triangles').textContent=String(data.mesh.triangles.length);$('surface-bound').textContent=data.error_bounds.reduce((a,b)=>Math.max(a,b),0).toExponential(3);$('quarters').textContent=String(data.rational_roof_path.quarters.length);$('path-degree').textContent='8';$('path-bound').textContent=data.rational_roof_path.quarters.reduce((a,q)=>q.error_bounds.reduce((x,e)=>Math.max(x,e),a),0).toExponential(3);$('status').textContent='✓ Exact rational roof path validated · stock unchanged · no bore created';
 }
 const update=()=>{try{generate();}catch(e){$('status').textContent='Roof path rejected: '+e.message+' Accepted stock and path are preserved.';}};
 $('apply').addEventListener('click',update);$('surface-error').addEventListener('input',update);$('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('reset').addEventListener('click',()=>viewer.reset());generate();window.haganeGraphRationalRoofCircle.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}

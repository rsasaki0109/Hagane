import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try {
 const viewer=createMeshViewer($('view'),{doubleSided:true});
 const response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');
 const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;
 window.haganeSurfaceTrim={ready:false,data:null};
 function generate(){
  const height=Number($('height').value),weight=Number($('weight').value),error=Number($('surface-error').value),ranges=[['u-min','u-max'],['v-min','v-max']].map(ids=>ids.map(id=>Number($(id).value)));
  // Query the rectangle's midpoint in the original source domain.
  const u=ranges[0][0]+(ranges[0][1]-ranges[0][0])/2,v=ranges[1][0]+(ranges[1][1]-ranges[1][0])/2;
  const status=k.hagane_generate_surface_trim(height,weight,u,v,error,...ranges.flat(),Number($('crease').checked));
  const data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);
  viewer.setMesh(data);const segments=[[data.point,data.point.map((x,i)=>x+data.normal[i]*14)]];
  for(const section of data.sections)for(let i=3;i<section.samples.length;i+=3)segments.push([section.samples.slice(i-3,i),section.samples.slice(i,i+3)]);
  viewer.setSegments(segments);window.haganeSurfaceTrim.data=data;
  $('height-label').value=height.toFixed(0);$('weight-label').value=weight.toFixed(2);$('surface-error-label').value=error.toFixed(2);$('domain').textContent=ranges.map(r=>'['+r.map(x=>x.toFixed(2)).join(', ')+']').join(' × ');$('selected-uv').textContent=`(${u.toFixed(3)}, ${v.toFixed(3)})`;$('patch-count').textContent=String(data.patch_ranges.length);$('cells').textContent=String(data.cells);$('surface-bound').textContent=Math.max(...data.cell_bounds).toExponential(3);$('status').textContent='✓ Exact restricted B-rep · original UV retained';
 }
 const update=()=>{try{generate();}catch(e){$('status').textContent=e.message;}};
 $('apply').addEventListener('click',update);for(const id of ['height','weight','surface-error'])$(id).addEventListener('input',update);$('crease').addEventListener('change',update);$('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('reset').addEventListener('click',()=>viewer.reset());generate();window.haganeSurfaceTrim.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}

import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try{
 const viewer=createMeshViewer($('view'),{doubleSided:true});
 const response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;
 window.haganeSurface={ready:false,data:null,setAngle:a=>viewer.setAngle(a)};
 function generate(){const height=Number($('height').value),weight=Number($('weight').value),u=Number($('u').value),v=Number($('v').value);const status=k.hagane_generate_surface(height,weight,u,v);const data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);window.haganeSurface.data=data;viewer.setMesh(data);const segments=[[data.point,data.point.map((x,i)=>x+data.normal[i]*14)]];for(const section of data.sections){for(let i=3;i<section.samples.length;i+=3)segments.push([section.samples.slice(i-3,i),section.samples.slice(i,i+3)]);}viewer.setSegments(segments);$('height-label').value=height.toFixed(1);$('weight-label').value=weight.toFixed(2);$('u-label').value=u.toFixed(2);$('v-label').value=v.toFixed(2);$('point').textContent='('+data.point.map(v=>v.toFixed(2)).join(', ')+')';$('normal').textContent='('+data.normal.map(v=>v.toFixed(3)).join(', ')+')';$('status').textContent='✓ Exact boundary + U/V curves extracted in Rust';}
 for(const id of ['height','weight','u','v'])$(id).addEventListener('input',()=>{try{generate();}catch(e){$('status').textContent=e.message;}});
 $('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('spin').addEventListener('change',()=>viewer.autoRotate=$('spin').checked);$('reset').addEventListener('click',()=>viewer.reset());generate();window.haganeSurface.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}

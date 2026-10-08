import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile, mkdtemp, rm } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import assert from 'node:assert/strict';
const root=new URL('../web/',import.meta.url);
const types={'/':'text/html','/index.html':'text/html','/app.js':'text/javascript','/style.css':'text/css','/hagane.wasm':'application/wasm','/nurbs.html':'text/html','/nurbs.js':'text/javascript','/surface.html':'text/html','/surface.js':'text/javascript','/viewer.js':'text/javascript'};
const server=createServer(async(req,res)=>{try{const path=new URL(req.url,'http://localhost').pathname;if(!types[path]){res.writeHead(404);res.end();return;}const data=await readFile(new URL(path==='/'?'index.html':path.slice(1),root));res.writeHead(200,{'Content-Type':types[path]});res.end(data);}catch{res.writeHead(500);res.end('Build the WASM module first.');}});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
let browser;
try {
 const executable=process.env.HAGANE_CHROMIUM ?? (existsSync('/usr/bin/chromium')?'/usr/bin/chromium':undefined);
 browser=await chromium.launch({executablePath:executable,headless:true,args:['--enable-unsafe-swiftshader','--use-gl=angle','--use-angle=swiftshader']});
 const page=await browser.newPage({viewport:{width:1440,height:900},deviceScaleFactor:1});const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(`http://127.0.0.1:${server.address().port}/`);await page.waitForFunction(()=>window.haganeDemo?.ready);
 const volume=await page.evaluate(()=>window.haganeDemo.mesh.volume);assert.ok(Math.abs(volume-(115200-Math.PI*196*24))<1e-8);
 const initial=await page.locator('canvas').screenshot();await page.locator('canvas').focus();await page.keyboard.press('ArrowRight');assert.notDeepEqual(await page.locator('canvas').screenshot(),initial);
 const canvas=await page.locator('canvas').boundingBox();await page.mouse.move(canvas.x+canvas.width/2,canvas.y+canvas.height/2);await page.mouse.down();await page.mouse.move(canvas.x+canvas.width/2+80,canvas.y+canvas.height/2+20,{steps:6});await page.mouse.up();const orbit=await page.locator('canvas').screenshot();assert.notDeepEqual(orbit,initial);
 await page.mouse.wheel(0,150);await page.waitForTimeout(100);assert.notDeepEqual(await page.locator('canvas').screenshot(),orbit);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-(115200-Math.PI*24**2*24))<1e-8);assert.match(await page.locator('#radius-label').textContent(),/24.0/);
 const solid=await page.locator('canvas').screenshot();await page.locator('#wire').check();assert.notDeepEqual(await page.locator('canvas').screenshot(),solid);await page.locator('#wire').uncheck();
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});await page.locator('#reset').click();
 assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);
 assert.deepEqual(errors,[]);
 const presets=[{volume:115200-Math.PI*196*24,faces:7,edges:15},{volume:115200-Math.PI*196*24,faces:10,edges:24},{volume:69336,faces:12,edges:30},{volume:Math.PI*704*24,faces:4,edges:6},{volume:115200-Math.PI*196*24,faces:10,edges:24},{volume:(4800-(4-Math.PI)*196)*24,faces:10,edges:24}];
 for(let id=0;id<6;id++){
  await page.locator('#preset').selectOption(String(id));const mesh=await page.evaluate(()=>window.haganeDemo.mesh);assert.ok(Math.abs(mesh.volume-presets[id].volume)<1e-8);assert.equal(mesh.faces,presets[id].faces);assert.equal(mesh.edges,presets[id].edges);assert.equal(await page.locator('#radius').isDisabled(),id===2);assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);
 }
 await page.locator('#preset').selectOption('5');
 assert.match(await page.locator('#radius-title').textContent(),/Corner radius/);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-(4800-(4-Math.PI)*576)*24)<1e-8);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-mixed')){await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/mixed-profile.png',import.meta.url).pathname});}
 await page.locator('#preset').selectOption('1');
 const unplaced=await page.evaluate(()=>window.haganeDemo.mesh);
 await page.locator('#preset').selectOption('4');
 const placed=await page.evaluate(()=>window.haganeDemo.mesh);
 assert.notDeepEqual(placed.positions,unplaced.positions);assert.notDeepEqual(placed.normals,unplaced.normals);
 if(process.argv.includes('--capture-placement')){await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/placement.png',import.meta.url).pathname});}
 await page.locator('#preset').selectOption('0');
 if(process.argv.includes('--capture')){
  const frames=await mkdtemp(join(tmpdir(),'hagane-demo-'));
  for(let i=0;i<64;i++){if(i%16===0)await page.locator('#preset').selectOption(String(Math.floor(i/16)));await page.evaluate(a=>window.haganeDemo.setAngle(a),0.65+0.8*(i%16)/16);await page.screenshot({path:join(frames,`frame-${String(i).padStart(3,'0')}.png`)});}
  const output=new URL('../docs/demo.gif',import.meta.url).pathname;
  const result=spawnSync('ffmpeg',['-y','-framerate','12','-i',join(frames,'frame-%03d.png'),'-filter_complex','[0:v]scale=1008:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=3','-loop','0',output],{encoding:'utf8'});
  if(result.status!==0)throw Error(result.stderr);console.log('Recorded actual WASM/WebGL demo to docs/demo.gif (64 frames, four exact B-rep operations).');await rm(frames,{recursive:true,force:true});
 }
 await page.setViewportSize({width:390,height:844});await page.waitForTimeout(100);assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);assert.deepEqual(errors,[]);
 await page.setViewportSize({width:1440,height:900});
 await page.goto(`http://127.0.0.1:${server.address().port}/nurbs.html`);await page.waitForFunction(()=>window.haganeNurbs?.ready);
 let curve=await page.evaluate(()=>window.haganeNurbs.data);assert.ok(Math.abs(curve.point[0]-Math.SQRT1_2)<1e-14);assert.ok(Math.abs(curve.point[1]-Math.SQRT1_2)<1e-14);
 const circleImage=await page.locator('canvas').screenshot();
 await page.locator('#weight').evaluate(e=>{e.value=2;e.dispatchEvent(new Event('input'));});
 curve=await page.evaluate(()=>window.haganeNurbs.data);assert.ok(Math.abs(curve.point[0]-5/6)<1e-14);assert.notDeepEqual(await page.locator('canvas').screenshot(),circleImage);
 await page.locator('#parameter').evaluate(e=>{e.value=0;e.dispatchEvent(new Event('input'));});curve=await page.evaluate(()=>window.haganeNurbs.data);assert.deepEqual(curve.point,[1,0,0]);assert.deepEqual(curve.tangent,[0,4,0]);
 await page.locator('#parameter').evaluate(e=>{e.value=0.5;e.dispatchEvent(new Event('input'));});await page.locator('#circle').click();
 curve=await page.evaluate(()=>window.haganeNurbs.data);assert.ok(Math.abs(curve.weight-Math.SQRT1_2)<1e-15);assert.ok(Math.abs(curve.point[0]-Math.SQRT1_2)<1e-14);
 if(process.argv.includes('--capture')){await page.screenshot({path:new URL('../docs/nurbs.png',import.meta.url).pathname});}
 await page.setViewportSize({width:390,height:844});await page.waitForTimeout(100);assert.ok(await page.locator('canvas').isVisible());assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);assert.deepEqual(errors,[]);
 await page.setViewportSize({width:1440,height:900});
 await page.goto(`http://127.0.0.1:${server.address().port}/surface.html`);await page.waitForFunction(()=>window.haganeSurface?.ready);
 let patch=await page.evaluate(()=>window.haganeSurface.data);assert.ok(Math.abs(patch.point[2]-8.75)<1e-13);assert.deepEqual(patch.normal,[0,0,1]);const initialPatch=await page.locator('canvas').screenshot();
 await page.locator('#height').evaluate(e=>{e.value=0;e.dispatchEvent(new Event('input'));});patch=await page.evaluate(()=>window.haganeSurface.data);assert.equal(patch.point[2],0);assert.notDeepEqual(await page.locator('canvas').screenshot(),initialPatch);
 await page.locator('#height').evaluate(e=>{e.value=35;e.dispatchEvent(new Event('input'));});await page.locator('#weight').evaluate(e=>{e.value=2;e.dispatchEvent(new Event('input'));});patch=await page.evaluate(()=>window.haganeSurface.data);assert.ok(Math.abs(patch.point[2]-14)<1e-13);
 await page.locator('#u').evaluate(e=>{e.value=0.3;e.dispatchEvent(new Event('input'));});await page.locator('#v').evaluate(e=>{e.value=0.7;e.dispatchEvent(new Event('input'));});patch=await page.evaluate(()=>window.haganeSurface.data);assert.ok(Math.abs(Math.hypot(...patch.normal)-1)<1e-13);assert.ok(Math.abs(patch.normal[0])+Math.abs(patch.normal[1])>0.01);
 const shaded=await page.locator('canvas').screenshot();await page.locator('#wire').check();assert.notDeepEqual(await page.locator('canvas').screenshot(),shaded);await page.locator('#wire').uncheck();await page.locator('canvas').focus();await page.keyboard.press('ArrowRight');assert.notDeepEqual(await page.locator('canvas').screenshot(),shaded);
 if(process.argv.includes('--capture')){await page.locator('#height').evaluate(e=>{e.value=60;e.dispatchEvent(new Event('input'));});await page.locator('#weight').evaluate(e=>{e.value=2;e.dispatchEvent(new Event('input'));});await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/nurbs-surface.png',import.meta.url).pathname});}
 assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);await page.setViewportSize({width:390,height:844});await page.waitForTimeout(100);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);assert.deepEqual(errors,[]);
 console.log('Browser: NURBS surface height/weight/UV, analytic point/normal, open-patch shading, wireframe, orbit and responsive layout passed.');
 console.log('Browser: NURBS weight/parameter controls, exact-circle reset, native-derived coordinates, canvas changes and responsive layout passed.');
 console.log('Browser: 6 B-rep presets, WASM generation, radius, keyboard/drag orbit, wheel zoom, wireframe, reset, responsive rendering passed.');
} finally {await browser?.close();server.close();}

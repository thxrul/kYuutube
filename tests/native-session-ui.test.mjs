import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const source=readFileSync(new URL('../web/native-account.js',import.meta.url),'utf8');
const html=readFileSync(new URL('../web/index.html',import.meta.url),'utf8');
const tick=()=>new Promise(setImmediate);
function screen(fail=false){
 const elements=new Map([...html.matchAll(/<[^>]+id="([^"]+)"[^>]*>/g)].map(([tag,id])=>[id,{hidden:tag.includes(' hidden'),disabled:tag.includes(' disabled'),textContent:'',showModal(){},close(){}}]));
 const calls=[];
 vm.runInNewContext(source,{document:{documentElement:{dataset:{}},getElementById:id=>elements.get(id)},CustomEvent:class{constructor(type,data){this.type=type;this.detail=data.detail;}},window:{dispatchEvent(){},__TAURI__:{core:{invoke:async(name,...args)=>{
 calls.push({name,args});if(name==='account_status')return {available:false,has_session:false};
 if(name==='finish_music_login'&&fail)throw new Error('YouTube Music did not verify this account');
 }}}}});
 return{get:id=>elements.get(id),calls};
}
test('native login opens a login window and requires explicit verified completion',async()=>{
 const ui=screen();await tick();assert.equal(ui.get('google-sign-in').disabled,false);
 await ui.get('google-sign-in').onclick();
 assert.equal(ui.get('finish-sign-in').hidden,false);
 assert.doesNotMatch(ui.get('welcome-heading').textContent,/signed in/);
 await ui.get('finish-sign-in').onclick();
 assert.match(ui.get('welcome-heading').textContent,/signed in/);
 assert.equal(ui.calls.find(c=>c.name==='finish_music_login').args.length,0);
});
test('failed account verification never claims login success',async()=>{
 const ui=screen(true);await tick();await ui.get('google-sign-in').onclick();await ui.get('finish-sign-in').onclick();
 assert.doesNotMatch(ui.get('welcome-heading').textContent,/signed in/);
 assert.equal(ui.get('finish-sign-in').hidden,false);
});
test('cancel closes login without creating a native session',async()=>{
 const ui=screen();await tick();await ui.get('google-sign-in').onclick();await ui.get('cancel-sign-in').onclick();
 assert.ok(ui.calls.some(c=>c.name==='cancel_music_login'));
 assert.equal(ui.get('finish-sign-in').hidden,true);
 assert.doesNotMatch(ui.get('welcome-heading').textContent,/signed in/);
});

import assert from 'node:assert/strict';
import {sqlCompletions,sqlReferencedTables} from '../src/capabilities/database/model/studioSql.mjs';
const catalog={users:['id','name','mobile'],orders:['id','user_id','total'],'app.users':['id','name','mobile'],'user profiles':['profile_id']};
let count=0;
function labels(query){const cursor=query.indexOf('|');return sqlCompletions(query.replace('|',''),cursor,Object.keys(catalog),[],catalog).map(i=>i.label);}
function same(query,expected){assert.deepEqual(labels(query),expected,query);count++;}
same('SELECT u.| FROM users AS u JOIN orders o ON u.id=o.user_id',['id','name','mobile']);
same('SELECT o.| FROM users u JOIN orders o ON u.id=o.user_id',['id','user_id','total']);
same('WITH recent AS (SELECT user_id, total AS amount FROM orders) SELECT r.| FROM recent r',['user_id','amount']);
same('WITH recent(customer, amount) AS (SELECT user_id,total FROM orders) SELECT r.| FROM recent r',['customer','amount']);
same('WITH recent AS (SELECT user_id,total FROM orders), copy AS (SELECT * FROM recent) SELECT c.| FROM copy c',['user_id','total']);
same('SELECT q.| FROM (SELECT id,name FROM users) q',['id','name']);
same('SELECT u.id FROM users u; SELECT u.| FROM orders u',['id','user_id','total']);
same('SELECT u.id FROM users u WHERE EXISTS (SELECT u.| FROM orders u)',['id','user_id','total']);
same('SELECT u.id FROM users u WHERE EXISTS (SELECT u.| FROM orders o)',['id','name','mobile']);
same('SELECT nope.| FROM users u',[]);
same("SELECT 'u.|' FROM users u",[]);
same('SELECT 1 -- u.|\nFROM users u',[]);
same('SELECT p.| FROM "user profiles" AS p',['profile_id']);
same('SELECT u.| FROM app.users u',['id','name','mobile']);
same('WITH u AS (SELECT * FROM users) SELECT u.| FROM u',['id','name','mobile']);
same('SELECT u.| FROM users u WHERE id IN (SELECT o.user_id FROM orders o)',['id','name','mobile']);
assert.ok(labels('WITH recent AS (SELECT id FROM orders) SELECT * FROM re|').includes('recent'));count++;
assert.deepEqual(sqlReferencedTables('WITH r AS (SELECT id FROM orders) SELECT r.id FROM r JOIN users u ON r.id=u.id'),['orders','users']);count++;
assert.deepEqual(sqlReferencedTables("SELECT 'FROM secrets' FROM users -- JOIN passwords\n"),['users']);count++;
assert.equal(sqlCompletions('SELECT mo FROM orders',9,Object.keys(catalog),['mobile'],catalog).some(item=>item.label==='mobile'),false);count++;
assert.deepEqual(sqlReferencedTables('SELECT id FROM users; SELECT id FROM orders'),['users','orders']);count++;
console.log(`SQL_COMPLETION_SCOPES=PASS assertions=${count} (bounded query scopes, not server semantic validation)`);

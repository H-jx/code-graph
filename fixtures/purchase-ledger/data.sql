INSERT INTO purchase_detail VALUES
 ('A','P1',1,'S1',10,500,'APPROVED'), ('A','P1',2,'S1',4,2000,'APPROVED'),
 ('A','P2',1,'S2',3,700,'APPROVED'), ('A','P3',1,'S1',9,900,'CANCELLED'),
 ('B','P1',1,'S1',100,100,'APPROVED');
INSERT INTO supplier VALUES ('A','S1','本租户供应商'), ('A','S2','无收货供应商'), ('B','S1','其他租户供应商');
INSERT INTO receipt VALUES
 ('R1','A','P1',1,3,'POSTED',0), ('R2','A','P1',1,2,'POSTED',0),
 ('R3','A','P1',1,9,'CANCELLED',0), ('R4','A','P1',1,20,'POSTED',1),
 ('R5','A','P1',2,1,'POSTED',0), ('RB','B','P1',1,80,'POSTED',0);
INSERT INTO return_line VALUES ('T1','A','P1',1,1,'POSTED',0), ('T2','A','P1',1,99,'CANCELLED',0);
INSERT INTO invoice VALUES ('I1','A','P1',1,800,'POSTED',0), ('I2','A','P1',1,1200,'POSTED',0), ('I3','A','P1',1,9900,'POSTED',1);

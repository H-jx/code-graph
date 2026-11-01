/**
 * 采购申请 MikroORM 实体
 * 对应数据库表: PurchaseRequest
 */
import {
  Entity,
  PrimaryKey,
  Property,
  OneToMany,
  Collection,
  BeforeCreate,
  ManyToOne,
  Index,
  Ref,
} from '@mikro-orm/core';
import { createId } from '@paralleldrive/cuid2';
import { PurchaseRequestItemOrmEntity } from './purchase-request-item.orm-entity';
import { User } from 'src/user/entities/user';
import { Transaction } from 'src/line-side-inventory/entities/transaction.entity';

/**
 * 采购申请 ORM 实体
 */
@Entity({ tableName: 'PurchaseRequest' })
@Index({ properties: ['applicant'] })
@Index({ properties: ['status'] })
@Index({ properties: ['departmentId'] })
export class PurchaseRequestOrmEntity {
  // ==================== 主键 ====================
  /** 采购申请ID */
  @PrimaryKey({ type: 'string' })
  id!: string;

  // ==================== 业务属性 ====================
  /** 采购申请单号（唯一） */
  @Property({ type: 'string', length: 50, unique: true })
  requestNo!: string;

  /** 费用所属部门ID */
  @Property({ type: 'string', length: 32 })
  departmentId!: string;

  /** 申请事由 */
  @Property({ type: 'string', length: 500 })
  reason!: string;

  /** 采购类型（从明细的采购方式获取） */
  @Property({ type: 'string', length: 20, default: '生产物资' })
  procurementType!: string;

  /** 采购申请状态 */
  @Property({ type: 'string', length: 30, default: 'PENDING_SUBMIT' })
  status!: string;

  /** 撤销原因 */
  @Property({ type: 'string', length: 500, nullable: true })
  cancelReason?: string;

  /** 审批通过时间 */
  @Property({ type: 'datetime', columnType: 'timestamptz', nullable: true })
  approvedAt?: Date;

  /** 删除时间（软删除） */
  @Property({ type: 'datetime', columnType: 'timestamptz', nullable: true })
  deletedAt?: Date;

  /** 创建时间 */
  @Property({ type: 'datetime', columnType: 'timestamptz', onCreate: () => new Date() })
  createdAt!: Date;

  /** 更新时间 */
  @Property({ type: 'datetime', columnType: 'timestamptz', onCreate: () => new Date(), onUpdate: () => new Date() })
  updatedAt!: Date;

  // ==================== 关联属性 ====================
  /** 关联：申请人 */
  @ManyToOne(() => User, { nullable: false })
  applicant!: Ref<User>;

  /** 关联：采购申请明细列表 */
  @OneToMany(() => PurchaseRequestItemOrmEntity, 'purchaseRequest')
  items = new Collection<PurchaseRequestItemOrmEntity>(this);

  /** 关联：线边库流水 */
  @OneToMany(() => Transaction, 'purchaseRequest')
  transactions = new Collection<Transaction>(this);

  // ==================== 生命周期钩子 ====================
  @BeforeCreate()
  onCreate() {
    if (!this.id) {
      this.id = createId();
    }
  }
}

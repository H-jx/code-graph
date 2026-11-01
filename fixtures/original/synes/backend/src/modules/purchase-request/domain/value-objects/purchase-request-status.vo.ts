/**
 * 采购申请状态值对象
 * 反映采购申请在审批流程中的阶段
 */
export class PurchaseRequestStatus {
  // === 属性 ===
  readonly value: string; // 值
  readonly label: string; // 标题

  // === 静态实例 ===
  /** 待提交 */
  static readonly PENDING_SUBMIT = new PurchaseRequestStatus(
    'PENDING_SUBMIT',
    '待提交',
  );
  /** 审批中 */
  static readonly PENDING_APPROVAL = new PurchaseRequestStatus(
    'PENDING_APPROVAL',
    '审批中',
  );
  /** 已批准 */
  static readonly APPROVED = new PurchaseRequestStatus('APPROVED', '已批准');
  /** 已关闭 */
  static readonly CLOSED = new PurchaseRequestStatus('CLOSED', '已关闭');
  /** 已删除 */
  static readonly DELETED = new PurchaseRequestStatus('DELETED', '已删除');

  // === 私有构造函数 ===
  private constructor(value: string, label: string) {
    this.value = value;
    this.label = label;
  }

  // === 静态方法 ===

  /**
   * 根据值获取实例
   */
  static fromValue(value: string): PurchaseRequestStatus | null {
    const map: Record<string, PurchaseRequestStatus> = {
      PENDING_SUBMIT: PurchaseRequestStatus.PENDING_SUBMIT,
      PENDING_APPROVAL: PurchaseRequestStatus.PENDING_APPROVAL,
      APPROVED: PurchaseRequestStatus.APPROVED,
      CLOSED: PurchaseRequestStatus.CLOSED,
      DELETED: PurchaseRequestStatus.DELETED,
    };
    return map[value] ?? null;
  }

  /**
   * 获取所有实例
   */
  static all(): PurchaseRequestStatus[] {
    return [
      PurchaseRequestStatus.PENDING_SUBMIT,
      PurchaseRequestStatus.PENDING_APPROVAL,
      PurchaseRequestStatus.APPROVED,
      PurchaseRequestStatus.CLOSED,
      PurchaseRequestStatus.DELETED,
    ];
  }

  // === 实例方法 ===

  /**
   * 判断是否相等
   */
  equals(other: PurchaseRequestStatus): boolean {
    return this.value === other.value;
  }

  /**
   * 转换为字符串
   */
  toString(): string {
    return this.value;
  }

  /**
   * 转换为对象
   */
  toJSON(): { value: string; label: string } {
    return {
      value: this.value,
      label: this.label,
    };
  }
}

/**
 * 采购申请工作流
 * 实现 ProcessInterface，处理采购申请的审批流程
 *
 * 审批流程：
 * 1. 生产计划员审批
 * 2. 生产部负责人审批
 * 3. 项目负责人审批
 * 4. 审批完成
 */
import { Injectable } from '@nestjs/common';
import { ProcessInterface } from 'src/workflow/process.interface';
import { EntityManager } from '@mikro-orm/postgresql';
import { WorkflowService } from 'src/workflow/services/workflow/workflow.service';
import { UserService } from 'src/user/services/user/user.service';
import { MsgService } from 'src/weixin/services/msg/msg.service';
import { PurchaseRequestRepository } from '../../infrastructure/repositories/purchase-request.repository';
import { PurchaseRequestItemRepository } from '../../infrastructure/repositories/purchase-request-item.repository';
import { ProcurementDivisionRepository } from '../../infrastructure/repositories/procurement-division.repository';
import { Process } from 'src/workflow/entities/process';
import { Task, TaskResult } from 'src/workflow/entities/task';
import { BaseTaskInput, SubmitTaskInput } from 'src/api/objects/task.object';
import { camelCase, get, isEmpty, upperFirst } from 'lodash';
import { GraphQLError } from 'graphql';
import { PurchaseRequestItemOrmEntity } from '../../infrastructure/entities/purchase-request-item.orm-entity';
import { ProjectOwnerQueryService } from 'src/modules/business/services/project-owner-query.service';
import { PurchaseRequestService } from '../../domain/services/purchase-request.service';
import { PurchaseRequestItemService } from '../../domain/services/purchase-request-item.service';
import { ProcurementDivisionService } from '../../domain/services/procurement-division.service';

/**
 * 采购申请工作流任务类型
 */
export enum PurchaseRequestTaskType {
  /** 提交申请 */
  RequestSubmit = 'purchase-request.request.submit',

  /** 生产计划员审批 */
  ProductionPlanApprove = 'purchase-request.production-plan.approve',

  /** 生产部负责人审批 */
  ProductionDepartmentApprove = 'purchase-request.production-department.approve',

  /** 项目负责人审批 */
  ProjectOwnerApprove = 'purchase-request.project-owner.approve',

  /** 审批完成 */
  Completed = 'purchase-request.completed',
}

/**
 * 采购申请工作流任务类型标签
 */
export const PurchaseRequestTaskTypeLabels = {
  [PurchaseRequestTaskType.RequestSubmit]: '采购申请-提交申请',
  [PurchaseRequestTaskType.ProductionPlanApprove]: '采购申请-生产计划员审批',
  [PurchaseRequestTaskType.ProductionDepartmentApprove]:
    '采购申请-生产部负责人审批',
  [PurchaseRequestTaskType.ProjectOwnerApprove]: '采购申请-项目负责人审批',
  [PurchaseRequestTaskType.Completed]: '采购申请-审批完成',
};

/**
 * 采购申请工作流唯一标识
 */
export const PurchaseRequestWorkflowSlug = 'purchase-request';

/**
 * 采购申请工作流
 */
@Injectable()
export class PurchaseRequestWorkflow implements ProcessInterface {
  slug: string = PurchaseRequestWorkflowSlug;

  constructor(
    private readonly purchaseRequestService: PurchaseRequestService,
    private readonly purchaseRequestItemService: PurchaseRequestItemService,
    private readonly purchaseRequestRepository: PurchaseRequestRepository,
    private readonly purchaseRequestItemRepository: PurchaseRequestItemRepository,
    private readonly procurementDivisionRepository: ProcurementDivisionRepository,
    private readonly workflowService: WorkflowService,
    private readonly userService: UserService,
    private readonly msgService: MsgService,
    private readonly procurementDivisionService: ProcurementDivisionService,
    private readonly projectOwnerQueryService: ProjectOwnerQueryService,
  ) { }

  /**
   * 创建审批任务
   * 如果申请人与审批人是同一人，则自动进行审批
   */
  async createTasks(
    em: EntityManager,
    process: Process,
    type: PurchaseRequestTaskType,
    assigneeIds: number[],
  ) {
    // 【步骤1】先正常创建所有任务
    const tasks = await this.workflowService.createTasks({
      processId: process.processId,
      type,
      assigneeIds,
    });

    // 【步骤2】获取采购申请
    const purchaseRequest = await this.purchaseRequestService.findByIdAsEntity(
      process.targetId,
    );
    const applicantId = purchaseRequest?.getApplicantId();

    // 【步骤3】检查任务中是否有审批人与申请人相同，如有则自动审批
    if (applicantId && tasks) {
      const selfApprovalTasks = tasks.filter(
        (task) => task.assigneeId === applicantId,
      );

      for (const task of selfApprovalTasks) {
        // 构造虚拟 user 和 input，调用对应的 submit 方法
        const virtualUser = { id: applicantId };
        const input: BaseTaskInput = {
          note: '系统自动审批（申请人与审批人为同一人）',
        };

        // 根据任务类型调用对应的 submit 方法
        const taskKey = upperFirst(
          camelCase(task.type.split('.').slice(1).join(' ')),
        );
        const submitTaskKey = 'submit' + taskKey;
        const submitMethod = get(this, submitTaskKey);
        if (submitMethod) {
          await submitMethod.bind(this)(
            em,
            process,
            task,
            virtualUser,
            input,
          );
        }

        if (task.type !== 'purchase-request.project-owner.approve') {
          tasks.forEach(task => task.completed = new Date())
        }
      }
    }

    return {
      tasks,
    };
  }

  /**
   * 创建审批流程
   * 当采购申请提交时调用
   */
  async createProcess(em: EntityManager, userId: number, entity: any) {
    const purchaseRequest = await this.purchaseRequestService.findByIdAsEntity(
      entity.id,
    );
    if (!purchaseRequest) {
      throw new GraphQLError('采购申请不存在');
    }

    const process = await this.workflowService.createProcess({
      name: this.slug,
      targetId: purchaseRequest.getId(),
    });

    // 创建已完成的任务（提交申请）
    await this.workflowService.createCompletedTask({
      processId: process.processId,
      type: PurchaseRequestTaskType.RequestSubmit,
      assigneeId: userId,
      result: TaskResult.SUBMIT,
    });

    // 查询生产计划员角色用户
    const users =
      await this.userService.getUsersByRoleWithoutTestUser('生产计划员');
    if (isEmpty(users)) {
      throw new GraphQLError('没有能够审批的生产计划员，请联系管理员');
    }

    await this.createTasks(
      em,
      process,
      PurchaseRequestTaskType.ProductionPlanApprove,
      users.map((user) => user.id),
    );

    return process;
  }

  /**
   * 执行审批任务
   * 处理审批通过或拒绝的逻辑
   */
  async run(
    em: EntityManager,
    process: Process,
    task: Task,
    user: any,
    input: SubmitTaskInput,
  ): Promise<void> {
    if (process.name !== this.slug) {
      throw new Error(
        `Invalid Workflow ${this.slug} for Process ${process.name}`,
      );
    }

    const taskKey = upperFirst(
      camelCase(task.type.split('.').slice(1).join(' ')),
    );

    const submitTaskKey = 'submit' + taskKey;
    const rejectTaskKey = 'reject' + taskKey;

    const submitPayload = get(input, submitTaskKey);
    const rejectPayload = get(input, rejectTaskKey);

    const submitMethod = get(this, submitTaskKey);
    const rejectMethod = get(this, rejectTaskKey);

    if (!submitPayload && !rejectPayload) {
      throw new Error(`Not Found Process Task Data for "${taskKey}"`);
    }

    if (submitPayload && rejectPayload) {
      throw new Error(`Invalid Process Task Data for "${taskKey}"`);
    }

    if (submitPayload) {
      if (!submitMethod) {
        throw new Error(`Not Found Task Method for "${submitTaskKey}"`);
      }
      await submitMethod.bind(this)(em, process, task, user, submitPayload);
    } else {
      if (!rejectMethod) {
        throw new Error(`Not Found Task Method for "${rejectTaskKey}"`);
      }
      await rejectMethod.bind(this)(em, process, task, user, rejectPayload);
    }
  }

  /**
   * 生产计划员-审批
   */
  async submitProductionPlanApprove(
    em: EntityManager,
    process: Process,
    task: Task,
    user: any,
    input: BaseTaskInput,
  ) {
    task.approve(input.note);
    await this.workflowService.cancelTasks(
      process.processId,
      PurchaseRequestTaskType.ProductionPlanApprove,
    );

    // 更新采购申请状态为审批中
    const purchaseRequest = await this.purchaseRequestService.findByIdAsEntity(
      process.targetId,
    );

    // 查询生产部负责人角色用户
    const users =
      await this.userService.getUsersByRoleWithoutTestUser('生产部负责人');
    if (isEmpty(users)) {
      throw new GraphQLError('没有能够审批的生产部负责人，请联系管理员');
    }

    await this.createTasks(
      em,
      process,
      PurchaseRequestTaskType.ProductionDepartmentApprove,
      users.map((u) => u.id),
    );
  }

  /**
   * 生产计划员-拒绝
   */
  async rejectProductionPlanApprove(
    em: EntityManager,
    process: Process,
    task: Task,
    user: any,
    input: BaseTaskInput,
  ) {
    await this.revertToPendingSubmit(
      em,
      process,
      task,
      user,
      PurchaseRequestTaskType.ProductionPlanApprove,
      input,
    );
  }

  /**
   * 生产部负责人-审批
   */
  async submitProductionDepartmentApprove(
    em: EntityManager,
    process: Process,
    task: Task,
    user: any,
    input: BaseTaskInput,
  ) {
    task.approve(input.note);
    await this.workflowService.cancelTasks(
      process.processId,
      PurchaseRequestTaskType.ProductionDepartmentApprove,
    );

    // 查询项目负责人（根据采购明细关联的项目）
    const purchaseRequest = await this.purchaseRequestService.findByIdAsEntity(
      process.targetId,
    );
    if (!purchaseRequest) {
      throw new GraphQLError('采购申请不存在');
    }

    const details =
      await this.purchaseRequestItemService.findByPurchaseRequestIdAsEntity(
        purchaseRequest.getId(),
      );
    const projectIds = details
      .flatMap((d) => d.getProjectIds())
      .filter((id) => id);

    if (isEmpty(projectIds)) {
      // 如果没有关联项目，直接完成审批
      await this.completeApproval(em, process, task);
      return;
    }

    // 查询项目负责人
    const projectOwners =
      await this.projectOwnerQueryService.getProjectOwnersByProjectIds(
        projectIds,
      );
    if (isEmpty(projectOwners)) {
      throw new GraphQLError('没有能够审批的项目负责人，请联系管理员');
    }

    await this.createTasks(
      em,
      process,
      PurchaseRequestTaskType.ProjectOwnerApprove,
      projectOwners.map((u) => u.id),
    );
  }

  /**
   * 生产部负责人-拒绝
   */
  async rejectProductionDepartmentApprove(
    em: EntityManager,
    process: Process,
    task: Task,
    user: any,
    input: BaseTaskInput,
  ) {
    await this.revertToPendingSubmit(
      em,
      process,
      task,
      user,
      PurchaseRequestTaskType.ProductionDepartmentApprove,
      input,
    );
  }

  /**
   * 项目负责人-审批（会签：所有项目负责人都审批通过才能继续）
   */
  async submitProjectOwnerApprove(
    em: EntityManager,
    process: Process,
    task: Task,
    user: any,
    input: BaseTaskInput,
  ) {
    // 1. 仅完成当前用户的任务（不是取消所有任务，会签逻辑）
    task.approve(input.note);

    // 2. 检查是否所有会签人都已完成审批
    const remainingTasks = await this.workflowService.findTodoTasksByProcessAndType(
      process.processId,
      PurchaseRequestTaskType.ProjectOwnerApprove,
    );

    // 3. 如果还有未完成的任务，等待其他审批人
    if (remainingTasks.length > 0) {
      return; // 会签未完成，等待其他人
    }

    // 4. 所有会签人都已审批通过，继续完成流程
    // 标记流程完成
    process.setComplete();

    // 更新采购申请状态为已批准
    const purchaseRequest = await this.purchaseRequestService.findByIdAsEntity(
      process.targetId,
    );
    if (purchaseRequest) {
      purchaseRequest.approve();
      await this.purchaseRequestRepository.save(purchaseRequest);

      // 更新所有明细状态为已批准
      const details =
        await this.purchaseRequestItemService.findByPurchaseRequestIdAsEntity(
          purchaseRequest.getId(),
        );
      for (const detail of details) {
        detail.approve();
        await this.purchaseRequestItemRepository.save(detail);

        // 自动分配采购员
        const itemTable = await em.findOne(PurchaseRequestItemOrmEntity, {
          id: detail.getId(),
        });
        
        if (itemTable) {
          await this.procurementDivisionService.autoAssignPurchaser(
            itemTable,
            detail.getMaterialId(),
          );
        }
      }

      // 若存在 fallback 自动分配的明细，按 PR 合并通知采购部负责人
      await this.notifyPurchasersIfNeeded(
        purchaseRequest.getRequestNo(),
      );
    }
  }

  /**
   * 项目负责人-拒绝
   */
  async rejectProjectOwnerApprove(
    em: EntityManager,
    process: Process,
    task: Task,
    user: any,
    input: BaseTaskInput,
  ) {
    await this.revertToPendingSubmit(
      em,
      process,
      task,
      user,
      PurchaseRequestTaskType.ProjectOwnerApprove,
      input,
    );
  }

  /**
   * 回退到待提交状态
   * 包含：取消所有任务、回退申请状态、回退明细状态
   */
  private async revertToPendingSubmit(
    em: EntityManager,
    process: Process,
    task: Task,
    user: any,
    taskType: PurchaseRequestTaskType,
    input: BaseTaskInput,
  ) {
    await this.workflowService.rejectTask({
      task,
      note: input.note,
      cancelType: taskType,
    });

    // 3. 回退采购申请状态
    const purchaseRequest = await this.purchaseRequestService.findByIdAsEntity(
      process.targetId,
    );
    if (purchaseRequest) {
      purchaseRequest.reject();
      await this.purchaseRequestRepository.save(purchaseRequest);

      // 4. 回退所有明细状态
      const details =
        await this.purchaseRequestItemService.findByPurchaseRequestIdAsEntity(
          purchaseRequest.getId(),
        );
      for (const detail of details) {
        detail.reject();
        await this.purchaseRequestItemRepository.save(detail);
      }
    }

    const submitter = await this.workflowService.findProcessSubmitter(process.processId);
    if (submitter?.wxUserId) {
      const msg = `${user.name}驳回了您的采购申请（${purchaseRequest?.getRequestNo() ?? ''}），原因：${input.note ?? ''}`;
      await this.msgService.sendAppMsg([submitter], msg);
    }
  }

  /**
   * 完成审批流程（当没有关联项目时直接完成）
   */
  private async completeApproval(
    em: EntityManager,
    process: Process,
    task: Task,
  ) {
    // 标记流程完成
    process.setComplete();

    // 更新采购申请状态为已批准
    const purchaseRequest = await this.purchaseRequestService.findByIdAsEntity(
      process.targetId,
    );
    if (purchaseRequest) {
      purchaseRequest.approve();
      await this.purchaseRequestRepository.save(purchaseRequest);

      // 更新所有明细状态为已批准
      const details =
        await this.purchaseRequestItemService.findByPurchaseRequestIdAsEntity(
          purchaseRequest.getId(),
        );
      for (const detail of details) {
        detail.approve();
        await this.purchaseRequestItemRepository.save(detail);

        // 与 submitProjectOwnerApprove 对齐：补上自动分配采购员
        const itemTable = await em.findOne(
          PurchaseRequestItemOrmEntity,
          { id: detail.getId() },
          { populate: ['material'] },
        );
        if (!itemTable) continue;
        if (itemTable) {
          await this.procurementDivisionService.autoAssignPurchaser(
            itemTable,
            detail.getMaterialId(),
          );
        }

      }

      // 按 PR 合并通知采购部负责人
      await this.notifyPurchasersIfNeeded(
        purchaseRequest.getRequestNo(),
      );
    }
  }

  /**
   * 审批通过后
   * 合并推送一条企微消息给「采购部负责人」角色下的用户，
   * 消息中附带物料名称 / 规格 / 需求数量。
   * 失败仅记日志，不阻断主审批流。
   *
   * 2026-10-09：原通知角色 '采购负责人' 已下线, 合并到 '采购部负责人'.
   */
  private async notifyPurchasersIfNeeded(
    requestNo: string,
  ): Promise<void> {

    try {
      const purchasers = await this.userService.getUsersByRole('采购部负责人');
      if (purchasers.length === 0) {
        return;
      }
      const content =
        `您有新采购申请单,请确认是否需配分采购员。\n` +
        `单号: ${requestNo}\n `;
      await this.msgService.sendAppMsg(purchasers, content);
    } catch (err) {
      // 不影响主审批流
      console.error('[PurchaseRequestWorkflow] 发送采购员分配通知失败', err);
    }
  }
}

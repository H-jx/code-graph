/**
 * 采购申请 GraphQL Resolver
 * 处理采购申请的查询和变更操作
 * 使用 CQRS 模式：通过 CommandBus/QueryBus 调用 Application 层 Handler
 */
import {
  Resolver,
  Query,
  Mutation,
  Args,
  ID,
  ResolveField,
  Parent,
} from '@nestjs/graphql';
import {
  NotFoundException,
} from '@nestjs/common';
import {
  EnsureRequestContext,
  MikroORM,
} from '@mikro-orm/postgresql';
import { CommandBus } from 'src/modules/event-bus/command/command-bus.service';
import { QueryBus } from 'src/modules/event-bus/query/query-bus.service';
import { Auth } from 'src/api/decorators/auth.decorator';
import {
  PurchaseRequestObject,
  PurchaseRequestConnectionObject,
} from '../objects';
import { ProcurementType } from '../enums/procurement-type.enum';
import {
  PurchaseRequestInput,
  UpdatePurchaseRequestInput,
  PurchaseRequestFilterInput,
} from '../inputs';
import { PurchaseRequestPermissionService } from 'src/modules/purchase-request/application/services/purchase-request-permission.service';
import { PagingParamInput } from 'src/common/graphql/paging-param.input';
import { FieldSortInput } from 'src/common/graphql/field-sort.input';
import {
  mapFilter,
  mapSort,
  mapPagination,
} from '../mappers/purchase-request-filter.mapper';
import {
  mapDtoToObject,
} from '../mappers/purchase-request-object.mapper';
import { CostDepartmentObject } from 'src/modules/cost-department/interface/graphql/types/cost-department.object';
import { mapEntityToObject as mapCostDepartmentToObject } from 'src/modules/cost-department/interface/graphql/mappers/cost-department-object.mapper';
import { CostDepartmentQueryService } from 'src/modules/cost-department/application/services/cost-department-query.service';
import { UserService } from 'src/user/services/user/user.service';
import { User } from 'src/user/entities/user';
import { UserId } from 'src/common/decorators/user.decorator';
import { Process } from 'src/api/objects/process.object';
import { WorkflowService } from 'src/workflow/services/workflow/workflow.service';
import { Task } from 'src/workflow/entities/task';
import { PurchaseRequestItemService } from '../../../domain/services/purchase-request-item.service';
import { PurchaseRequestItemObject } from '../objects/purchase-request-item.object';
import { mapItemEntityToObject } from '../mappers/purchase-request-item-object.mapper';

// Application Layer Commands & Queries
import {
  CreatePurchaseRequestCommand,
  UpdatePurchaseRequestCommand,
  DeletePurchaseRequestCommand,
  SubmitPurchaseRequestCommand,
  CancelPurchaseRequestCommand,
} from '../../../application/commands';
import {
  GetPurchaseRequestDetailQuery,
  GetPurchaseRequestListQuery,
} from '../../../application/queries';
import { PurchaseRequestWorkflow } from 'src/modules/purchase-request/application/workflows/purchase-request.workflow';

@Resolver(() => PurchaseRequestObject)
export class PurchaseRequestResolver {
  constructor(
    private readonly orm: MikroORM,
    private readonly commandBus: CommandBus,
    private readonly queryBus: QueryBus,
    private readonly costDepartmentQueryService: CostDepartmentQueryService,
    private readonly userService: UserService,
    private readonly purchaseRequestWorkflow: PurchaseRequestWorkflow,
    private readonly workflowService: WorkflowService,
    private readonly permissionService: PurchaseRequestPermissionService,
    private readonly purchaseRequestItemService: PurchaseRequestItemService,
  ) {}

  // ==================== Query ====================

  /**
   * 根据 ID 查询采购申请详情
   */
  @Query(() => PurchaseRequestObject, { description: '根据 ID 查询采购申请' })
  @Auth()
  @EnsureRequestContext()
  async purchaseRequest(
    @Args('id', { type: () => ID }) id: string,
    @UserId() userId: number,
  ): Promise<PurchaseRequestObject> {
    const pr = await this.queryBus.execute(
      new GetPurchaseRequestDetailQuery({ id }),
    );

    if (!pr) {
      throw new NotFoundException('采购申请不存在');
    }

    return mapDtoToObject(pr);
  }

  /**
   * 查询采购申请列表（支持分页、筛选、排序）
   */
  @Query(() => PurchaseRequestConnectionObject, {
    description: '查询采购申请列表',
  })
  @Auth()
  @EnsureRequestContext()
  async purchaseRequests(
    @UserId() userId: number,
    @Args('filter', { nullable: true }) filter?: PurchaseRequestFilterInput,
    @Args('sort', { type: () => [FieldSortInput], nullable: true })
    sort?: FieldSortInput[],
    @Args('pagination', { type: () => PagingParamInput, nullable: true })
    pagination?: PagingParamInput,
  ): Promise<PurchaseRequestConnectionObject> {
    // 构建查询条件
    const mappedFilter = mapFilter(filter);

    // 检查是否有权限查看所有记录
    const canViewAll = await this.permissionService.canViewAll(userId);

    if (!canViewAll) {
      const leaderDepartmentIds: string[] = [];
      const approverRequestIds =
        await this.permissionService.getApproverPurchaseRequestIds(userId);
      // 2026-10-15：直接取负责人项目集合下推（原先只下推布尔 isProjectOwner，
      // 由 repository 另查 ProjectMember 展开项目，两者数据源不一致）。
      const ownerProjectIds =
        await this.permissionService.getOwnerProjectIds(userId);

      const hasAnyPermission =
        leaderDepartmentIds.length > 0 ||
        approverRequestIds.length > 0 ||
        ownerProjectIds.length > 0;

      if (hasAnyPermission) {
        mappedFilter.authorizedQuery = {
          departmentIds:
            leaderDepartmentIds.length > 0 ? leaderDepartmentIds : undefined,
          approverRequestIds:
            approverRequestIds.length > 0 ? approverRequestIds : undefined,
          ownerProjectIds:
            ownerProjectIds.length > 0 ? ownerProjectIds : undefined,
          userId: userId,
        };
      } else {
        mappedFilter.applicantId = userId;
      }
    }

    const result = await this.queryBus.execute(
      new GetPurchaseRequestListQuery({
        filter: mappedFilter,
        sort: mapSort(sort),
        pagination: mapPagination(pagination),
      }),
    );

    return {
      totalCount: result.totalCount,
      nodes: result.nodes.map((n: any) => mapDtoToObject(n)),
    };
  }

  // ==================== Mutation ====================

  /**
   * 创建采购申请（支持创建后直接提交审批）
   */
  @Auth()
  @Mutation(() => PurchaseRequestObject, { description: '创建采购申请' })
  @EnsureRequestContext()
  async createPurchaseRequest(
    @Args('input') input: PurchaseRequestInput,
    @UserId() userId: number,
  ): Promise<PurchaseRequestObject> {
    const command = new CreatePurchaseRequestCommand({
      departmentId: input.departmentId,
      reason: input.reason,
      details: input.details?.map((d) => ({
        purchaseRequisitionFormIds: d.purchaseRequisitionFormIds,
        procurementMethod: d.procurementMethod,
        materialId: d.materialId,
        projectIds: d.projectIds,
        requiredQuantity: d.requiredQuantity,
        expectedDeliveryDate: d.expectedDeliveryDate,
        remark: d.remark,
        fileIds: d.fileIds,
        specifiedMaterials: d.specifiedMaterials?.map((s) => ({
          materialId: s.materialId,
          materialName: s.materialName,
          specification: s.specification,
          unit: s.unit,
          batch: s.batch,
        })),
      })),
      submitAfterCreate: input.submitAfterCreate,
    });
    // 将 userId 附加到 command
    (command as any).userId = userId;

    const id = await this.commandBus.execute(command);

    // 查询返回创建的采购申请
    const pr = await this.queryBus.execute(
      new GetPurchaseRequestDetailQuery({ id }),
    );

    return mapDtoToObject(pr);
  }

  /**
   * 更新采购申请（仅草稿状态可编辑）
   */
  @Auth()
  @Mutation(() => PurchaseRequestObject, { description: '更新采购申请' })
  @EnsureRequestContext()
  async updatePurchaseRequest(
    @Args('id', { type: () => ID }) id: string,
    @Args('input') input: UpdatePurchaseRequestInput,
    @UserId() userId: number,
  ): Promise<PurchaseRequestObject> {
    const command = new UpdatePurchaseRequestCommand({
      id,
      departmentId: input.departmentId,
      reason: input.reason,
      details: input.details?.map((d) => ({
        id: d.id,
        purchaseRequisitionFormIds: d.purchaseRequisitionFormIds,
        procurementMethod: d.procurementMethod,
        materialId: d.materialId,
        projectIds: d.projectIds,
        requiredQuantity: d.requiredQuantity,
        expectedDeliveryDate: d.expectedDeliveryDate,
        remark: d.remark,
        fileIds: d.fileIds,
        specifiedMaterials: d.specifiedMaterials?.map((s) => ({
          materialId: s.materialId,
          materialName: s.materialName,
          specification: s.specification,
          unit: s.unit,
          batch: s.batch,
        })),
      })),
    });
    (command as any).userId = userId;

    await this.commandBus.execute(command);

    // 查询返回更新的采购申请
    const pr = await this.queryBus.execute(
      new GetPurchaseRequestDetailQuery({ id }),
    );

    return mapDtoToObject(pr);
  }

  /**
   * 提交采购申请（触发工作流）
   */
  @Auth()
  @Mutation(() => PurchaseRequestObject, { description: '提交采购申请' })
  @EnsureRequestContext()
  async submitPurchaseRequest(
    @Args('id', { type: () => ID }) id: string,
    @UserId() userId: number,
  ): Promise<PurchaseRequestObject> {
    const command = new SubmitPurchaseRequestCommand({ id });
    (command as any).userId = userId;

    await this.commandBus.execute(command);

    // 创建工作流进程
    const em = this.orm.em.fork();
    await this.purchaseRequestWorkflow.createProcess(em, userId, { id });

    // 查询返回更新的采购申请
    const pr = await this.queryBus.execute(
      new GetPurchaseRequestDetailQuery({ id }),
    );

    return mapDtoToObject(pr);
  }

  /**
   * 撤销采购申请
   */
  @Auth()
  @Mutation(() => PurchaseRequestObject, { description: '撤销采购申请' })
  @EnsureRequestContext()
  async cancelPurchaseRequest(
    @Args('id', { type: () => ID }) id: string,
    @Args('reason', { type: () => String }) reason: string,
    @UserId() userId: number,
  ): Promise<PurchaseRequestObject> {
    const command = new CancelPurchaseRequestCommand({ id, reason });
    (command as any).userId = userId;

    await this.commandBus.execute(command);

    // 查询返回更新的采购申请
    const pr = await this.queryBus.execute(
      new GetPurchaseRequestDetailQuery({ id }),
    );

    return mapDtoToObject(pr);
  }

  /**
   * 删除采购申请（仅草稿/已撤销状态可删除）
   */
  @Auth()
  @Mutation(() => Boolean, { description: '删除采购申请' })
  @EnsureRequestContext()
  async deletePurchaseRequest(
    @Args('id', { type: () => ID }) id: string,
    @UserId() userId: number,
  ): Promise<boolean> {
    const command = new DeletePurchaseRequestCommand({ id });
    (command as any).userId = userId;

    await this.commandBus.execute(command);
    return true;
  }

  // ==================== ResolveField ====================

  /**
   * 解析费用所属部门
   */
  @ResolveField(() => CostDepartmentObject, {
    name: 'department',
    nullable: true,
  })
  @EnsureRequestContext()
  async resolveDepartment(
    @Parent() pr: PurchaseRequestObject,
  ): Promise<CostDepartmentObject | null> {
    const department = await this.costDepartmentQueryService.findById(
      pr.departmentId,
    );
    if (!department) {
      return null;
    }
    return mapCostDepartmentToObject(department);
  }

  /**
   * 解析申请人
   */
  @ResolveField(() => User, { name: 'applicant', nullable: true })
  @EnsureRequestContext()
  async resolveApplicant(
    @Parent() pr: PurchaseRequestObject,
  ): Promise<User | null> {
    const user = await this.userService.findById(pr.applicantId);
    return user ?? null;
  }

  /**
   * 解析工作流进程
   */
  @ResolveField(() => Process, {
    name: 'process',
    nullable: true,
    description: '当前工作流进程',
  })
  @EnsureRequestContext()
  async resolveProcess(
    @Parent() pr: PurchaseRequestObject,
  ): Promise<Process | null> {
    try {
      const process = await this.workflowService.findProcessByTargetId(pr.id);
      if (!process) {
        return null;
      }
      const obj = new Process();
      obj.processId = process.processId;
      obj.name = process.name;
      obj.targetId = process.targetId ?? null;
      obj.targetFormId = process.targetFormId ?? null;
      obj.formId = process.formId ?? null;
      obj.variables = process.variables ?? null;
      obj.completed = process.completed ?? null;
      obj.disabled = process.disabled ?? null;
      obj.createdAt = process.createdAt;
      return obj;
    } catch {
      return null;
    }
  }

  /**
   * 解析工作流进程
   */
  @ResolveField(() => [Task], {
    name: 'tasks',
    description: '历史任务',
  })
  @EnsureRequestContext()
  async resolveTasks(@Parent() pr: PurchaseRequestObject): Promise<Task[]> {
    const em = this.orm.em;
    return await em.find(
      Task,
      {
        process: {
          targetId: pr.id,
        },
        decided: {
          $ne: null,
        },
      },
      {
        orderBy: {
          decided: 'DESC',
        },
      },
    );
  }

  /**
   * 解析采购类型（枚举）
   */
  @ResolveField(() => ProcurementType, {
    name: 'procurementType',
    description: '采购类型（枚举）',
  })
  procurementType(): ProcurementType {
    return ProcurementType.RAW_MATERIAL;
  }

  /**
   * 解析采购申请明细列表
   */
  @ResolveField(() => [PurchaseRequestItemObject], {
    name: 'details',
    description: '采购申请明细列表',
    nullable: true,
  })
  @EnsureRequestContext()
  async resolveDetails(
    @Parent() pr: PurchaseRequestObject,
  ): Promise<PurchaseRequestItemObject[]> {
    const items = await this.purchaseRequestItemService.findByPurchaseRequestId(
      pr.id,
    );
    return items.map((item) => mapItemEntityToObject(item));
  }
}